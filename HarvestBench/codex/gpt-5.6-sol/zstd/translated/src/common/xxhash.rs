extern "C" {
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
}
pub type size_t = usize;
pub type XXH_errorcode = ::core::ffi::c_uint;
pub const XXH_ERROR: XXH_errorcode = 1;
pub const XXH_OK: XXH_errorcode = 0;
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type XXH32_hash_t = uint32_t;
pub type xxh_u32 = XXH32_hash_t;
pub type XXH_alignment = ::core::ffi::c_uint;
pub const XXH_unaligned: XXH_alignment = 1;
pub const XXH_aligned: XXH_alignment = 0;
pub type xxh_u8 = uint8_t;
pub type xxh_unalign32 = xxh_u32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct XXH32_state_s {
    pub total_len_32: XXH32_hash_t,
    pub large_len: XXH32_hash_t,
    pub v: [XXH32_hash_t; 4],
    pub mem32: [XXH32_hash_t; 4],
    pub memsize: XXH32_hash_t,
    pub reserved: XXH32_hash_t,
}
pub type XXH32_state_t = XXH32_state_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct XXH32_canonical_t {
    pub digest: [::core::ffi::c_uchar; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct xxh_sa {
    pub x: [::core::ffi::c_char; 1],
}
pub type XXH64_hash_t = uint64_t;
pub type xxh_u64 = XXH64_hash_t;
pub type xxh_unalign64 = xxh_u64;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct XXH64_state_s {
    pub total_len: XXH64_hash_t,
    pub v: [XXH64_hash_t; 4],
    pub mem64: [XXH64_hash_t; 4],
    pub memsize: XXH32_hash_t,
    pub reserved32: XXH32_hash_t,
    pub reserved64: XXH64_hash_t,
}
pub type XXH64_state_t = XXH64_state_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct XXH64_canonical_t {
    pub digest: [::core::ffi::c_uchar; 8],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct xxh_sa_0 {
    pub x: [::core::ffi::c_char; 1],
}
pub const XXH_VERSION_MAJOR: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const XXH_VERSION_MINOR: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const XXH_VERSION_RELEASE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const XXH_VERSION_NUMBER: ::core::ffi::c_int =
    XXH_VERSION_MAJOR * 100 as ::core::ffi::c_int * 100 as ::core::ffi::c_int
        + XXH_VERSION_MINOR * 100 as ::core::ffi::c_int
        + XXH_VERSION_RELEASE;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const XXH_FORCE_ALIGN_CHECK: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const XXH32_ENDJMP: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
unsafe extern "C" fn XXH_malloc(mut s: size_t) -> *mut ::core::ffi::c_void {
    return malloc(s);
}
unsafe extern "C" fn XXH_free(mut p: *mut ::core::ffi::c_void) {
    free(p);
}
unsafe extern "C" fn XXH_memcpy(
    mut dest: *mut ::core::ffi::c_void,
    mut src: *const ::core::ffi::c_void,
    mut size: size_t,
) -> *mut ::core::ffi::c_void {
    return memcpy(dest, src, size);
}
unsafe extern "C" fn XXH_read32(mut ptr: *const ::core::ffi::c_void) -> xxh_u32 {
    return *(ptr as *const xxh_unalign32);
}
pub const XXH_CPU_LITTLE_ENDIAN: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
unsafe extern "C" fn XXH_swap32(mut x: xxh_u32) -> xxh_u32 {
    return x << 24 as ::core::ffi::c_int & 0xff000000 as xxh_u32
        | x << 8 as ::core::ffi::c_int & 0xff0000 as xxh_u32
        | x >> 8 as ::core::ffi::c_int & 0xff00 as xxh_u32
        | x >> 24 as ::core::ffi::c_int & 0xff as xxh_u32;
}
#[inline(always)]
unsafe extern "C" fn XXH_readLE32(mut ptr: *const ::core::ffi::c_void) -> xxh_u32 {
    return if XXH_CPU_LITTLE_ENDIAN != 0 {
        XXH_read32(ptr)
    } else {
        XXH_swap32(XXH_read32(ptr))
    };
}
unsafe extern "C" fn XXH_readBE32(mut ptr: *const ::core::ffi::c_void) -> xxh_u32 {
    return if XXH_CPU_LITTLE_ENDIAN != 0 {
        XXH_swap32(XXH_read32(ptr))
    } else {
        XXH_read32(ptr)
    };
}
#[inline(always)]
unsafe extern "C" fn XXH_readLE32_align(
    mut ptr: *const ::core::ffi::c_void,
    mut align: XXH_alignment,
) -> xxh_u32 {
    if align as ::core::ffi::c_uint == XXH_unaligned as ::core::ffi::c_int as ::core::ffi::c_uint {
        return XXH_readLE32(ptr);
    } else {
        return if XXH_CPU_LITTLE_ENDIAN != 0 {
            *(ptr as *const xxh_u32)
        } else {
            XXH_swap32(*(ptr as *const xxh_u32))
        };
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH_versionNumber() -> ::core::ffi::c_uint {
    return XXH_VERSION_NUMBER as ::core::ffi::c_uint;
}
pub const XXH_PRIME32_1: ::core::ffi::c_uint = 0x9e3779b1 as ::core::ffi::c_uint;
pub const XXH_PRIME32_2: ::core::ffi::c_uint = 0x85ebca77 as ::core::ffi::c_uint;
pub const XXH_PRIME32_3: ::core::ffi::c_uint = 0xc2b2ae3d as ::core::ffi::c_uint;
pub const XXH_PRIME32_4: ::core::ffi::c_uint = 0x27d4eb2f as ::core::ffi::c_uint;
pub const XXH_PRIME32_5: ::core::ffi::c_uint = 0x165667b1 as ::core::ffi::c_uint;
unsafe extern "C" fn XXH32_round(mut acc: xxh_u32, mut input: xxh_u32) -> xxh_u32 {
    acc = (acc as ::core::ffi::c_uint)
        .wrapping_add(input.wrapping_mul(XXH_PRIME32_2 as xxh_u32) as ::core::ffi::c_uint)
        as xxh_u32 as xxh_u32;
    acc = acc.rotate_left(13 as ::core::ffi::c_int as ::core::ffi::c_uint as u32) as xxh_u32;
    acc = (acc as ::core::ffi::c_uint).wrapping_mul(XXH_PRIME32_1) as xxh_u32 as xxh_u32;
    return acc;
}
unsafe extern "C" fn XXH32_avalanche(mut hash: xxh_u32) -> xxh_u32 {
    hash = (hash as ::core::ffi::c_uint ^ (hash >> 15 as ::core::ffi::c_int) as ::core::ffi::c_uint)
        as xxh_u32;
    hash = (hash as ::core::ffi::c_uint).wrapping_mul(XXH_PRIME32_2) as xxh_u32 as xxh_u32;
    hash = (hash as ::core::ffi::c_uint ^ (hash >> 13 as ::core::ffi::c_int) as ::core::ffi::c_uint)
        as xxh_u32;
    hash = (hash as ::core::ffi::c_uint).wrapping_mul(XXH_PRIME32_3) as xxh_u32 as xxh_u32;
    hash = (hash as ::core::ffi::c_uint ^ (hash >> 16 as ::core::ffi::c_int) as ::core::ffi::c_uint)
        as xxh_u32;
    return hash;
}
unsafe extern "C" fn XXH32_finalize(
    mut hash: xxh_u32,
    mut ptr: *const xxh_u8,
    mut len: size_t,
    mut align: XXH_alignment,
) -> xxh_u32 {
    if ptr.is_null() {
        if !(len == 0 as size_t) {
            unreachable!();
        }
    }
    if XXH32_ENDJMP == 0 {
        len = (len as ::core::ffi::c_ulong & 15 as ::core::ffi::c_ulong) as size_t;
        while len >= 4 as size_t {
            hash = (hash as ::core::ffi::c_uint).wrapping_add(
                XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                    .wrapping_mul(XXH_PRIME32_3 as xxh_u32) as ::core::ffi::c_uint,
            ) as xxh_u32 as xxh_u32;
            ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
            hash = hash
                .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
            len = (len as ::core::ffi::c_ulong).wrapping_sub(4 as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
        while len > 0 as size_t {
            let fresh0 = ptr;
            ptr = ptr.offset(1);
            hash = (hash as ::core::ffi::c_uint)
                .wrapping_add((*fresh0 as ::core::ffi::c_uint).wrapping_mul(XXH_PRIME32_5))
                as xxh_u32 as xxh_u32;
            hash = hash
                .rotate_left(11 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                .wrapping_mul(XXH_PRIME32_1) as xxh_u32;
            len = len.wrapping_sub(1);
        }
        return XXH32_avalanche(hash);
    } else {
        's_499: {
            let mut current_block_122: u64;
            match len & 15 as size_t {
                12 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    current_block_122 = 1602911237003022065;
                }
                8 => {
                    current_block_122 = 1602911237003022065;
                }
                4 => {
                    current_block_122 = 1999811434528194146;
                }
                13 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    current_block_122 = 12852799825332870652;
                }
                9 => {
                    current_block_122 = 12852799825332870652;
                }
                5 => {
                    current_block_122 = 4990779174516811319;
                }
                14 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    current_block_122 = 6175455253125813810;
                }
                10 => {
                    current_block_122 = 6175455253125813810;
                }
                6 => {
                    current_block_122 = 12225724121032071869;
                }
                15 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    current_block_122 = 12512401675314337886;
                }
                11 => {
                    current_block_122 = 12512401675314337886;
                }
                7 => {
                    current_block_122 = 12259350387199304931;
                }
                3 => {
                    current_block_122 = 6721905202649677722;
                }
                2 => {
                    current_block_122 = 16954998436242079140;
                }
                1 => {
                    current_block_122 = 11896232456515752312;
                }
                0 => {
                    current_block_122 = 14965632903131415258;
                }
                _ => {
                    break 's_499;
                }
            }
            match current_block_122 {
                1602911237003022065 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    current_block_122 = 1999811434528194146;
                }
                12852799825332870652 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    current_block_122 = 4990779174516811319;
                }
                6175455253125813810 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    current_block_122 = 12225724121032071869;
                }
                12512401675314337886 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    current_block_122 = 12259350387199304931;
                }
                _ => {}
            }
            match current_block_122 {
                12225724121032071869 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    let fresh2 = ptr;
                    ptr = ptr.offset(1);
                    hash = (hash as ::core::ffi::c_uint)
                        .wrapping_add((*fresh2 as ::core::ffi::c_uint).wrapping_mul(XXH_PRIME32_5))
                        as xxh_u32 as xxh_u32;
                    hash = hash
                        .rotate_left(11 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_1) as xxh_u32;
                    let fresh3 = ptr;
                    ptr = ptr.offset(1);
                    hash = (hash as ::core::ffi::c_uint)
                        .wrapping_add((*fresh3 as ::core::ffi::c_uint).wrapping_mul(XXH_PRIME32_5))
                        as xxh_u32 as xxh_u32;
                    hash = hash
                        .rotate_left(11 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_1) as xxh_u32;
                    return XXH32_avalanche(hash);
                }
                4990779174516811319 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    let fresh1 = ptr;
                    ptr = ptr.offset(1);
                    hash = (hash as ::core::ffi::c_uint)
                        .wrapping_add((*fresh1 as ::core::ffi::c_uint).wrapping_mul(XXH_PRIME32_5))
                        as xxh_u32 as xxh_u32;
                    hash = hash
                        .rotate_left(11 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_1) as xxh_u32;
                    return XXH32_avalanche(hash);
                }
                1999811434528194146 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    return XXH32_avalanche(hash);
                }
                12259350387199304931 => {
                    hash = (hash as ::core::ffi::c_uint).wrapping_add(
                        XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align)
                            .wrapping_mul(XXH_PRIME32_3 as xxh_u32)
                            as ::core::ffi::c_uint,
                    ) as xxh_u32 as xxh_u32;
                    ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
                    hash = hash
                        .rotate_left(17 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_4) as xxh_u32;
                    current_block_122 = 6721905202649677722;
                }
                _ => {}
            }
            match current_block_122 {
                6721905202649677722 => {
                    let fresh4 = ptr;
                    ptr = ptr.offset(1);
                    hash = (hash as ::core::ffi::c_uint)
                        .wrapping_add((*fresh4 as ::core::ffi::c_uint).wrapping_mul(XXH_PRIME32_5))
                        as xxh_u32 as xxh_u32;
                    hash = hash
                        .rotate_left(11 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_1) as xxh_u32;
                    current_block_122 = 16954998436242079140;
                }
                _ => {}
            }
            match current_block_122 {
                16954998436242079140 => {
                    let fresh5 = ptr;
                    ptr = ptr.offset(1);
                    hash = (hash as ::core::ffi::c_uint)
                        .wrapping_add((*fresh5 as ::core::ffi::c_uint).wrapping_mul(XXH_PRIME32_5))
                        as xxh_u32 as xxh_u32;
                    hash = hash
                        .rotate_left(11 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_1) as xxh_u32;
                    current_block_122 = 11896232456515752312;
                }
                _ => {}
            }
            match current_block_122 {
                11896232456515752312 => {
                    let fresh6 = ptr;
                    ptr = ptr.offset(1);
                    hash = (hash as ::core::ffi::c_uint)
                        .wrapping_add((*fresh6 as ::core::ffi::c_uint).wrapping_mul(XXH_PRIME32_5))
                        as xxh_u32 as xxh_u32;
                    hash = hash
                        .rotate_left(11 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
                        .wrapping_mul(XXH_PRIME32_1) as xxh_u32;
                }
                _ => {}
            }
            return XXH32_avalanche(hash);
        }
        if 0 as ::core::ffi::c_int == 0 {
            unreachable!();
        }
        return hash;
    };
}
#[inline(always)]
unsafe extern "C" fn XXH32_endian_align(
    mut input: *const xxh_u8,
    mut len: size_t,
    mut seed: xxh_u32,
    mut align: XXH_alignment,
) -> xxh_u32 {
    let mut h32: xxh_u32 = 0;
    if input.is_null() {
        if !(len == 0 as size_t) {
            unreachable!();
        }
    }
    if len >= 16 as size_t {
        let bEnd: *const xxh_u8 = input.offset(len as isize);
        let limit: *const xxh_u8 = bEnd.offset(-(15 as ::core::ffi::c_int as isize));
        let mut v1: xxh_u32 = seed
            .wrapping_add(XXH_PRIME32_1 as xxh_u32)
            .wrapping_add(XXH_PRIME32_2 as xxh_u32);
        let mut v2: xxh_u32 = seed.wrapping_add(XXH_PRIME32_2 as xxh_u32);
        let mut v3: xxh_u32 = seed.wrapping_add(0 as xxh_u32);
        let mut v4: xxh_u32 = seed.wrapping_sub(XXH_PRIME32_1 as xxh_u32);
        loop {
            v1 = XXH32_round(
                v1,
                XXH_readLE32_align(input as *const ::core::ffi::c_void, align),
            );
            input = input.offset(4 as ::core::ffi::c_int as isize);
            v2 = XXH32_round(
                v2,
                XXH_readLE32_align(input as *const ::core::ffi::c_void, align),
            );
            input = input.offset(4 as ::core::ffi::c_int as isize);
            v3 = XXH32_round(
                v3,
                XXH_readLE32_align(input as *const ::core::ffi::c_void, align),
            );
            input = input.offset(4 as ::core::ffi::c_int as isize);
            v4 = XXH32_round(
                v4,
                XXH_readLE32_align(input as *const ::core::ffi::c_void, align),
            );
            input = input.offset(4 as ::core::ffi::c_int as isize);
            if !(input < limit) {
                break;
            }
        }
        h32 = v1
            .rotate_left(1 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
            .wrapping_add(v2.rotate_left(7 as ::core::ffi::c_int as ::core::ffi::c_uint as u32))
            .wrapping_add(v3.rotate_left(12 as ::core::ffi::c_int as ::core::ffi::c_uint as u32))
            .wrapping_add(v4.rotate_left(18 as ::core::ffi::c_int as ::core::ffi::c_uint as u32))
            as xxh_u32;
    } else {
        h32 = seed.wrapping_add(XXH_PRIME32_5 as xxh_u32);
    }
    h32 = (h32 as ::core::ffi::c_uint).wrapping_add(len as xxh_u32 as ::core::ffi::c_uint)
        as xxh_u32 as xxh_u32;
    return XXH32_finalize(h32, input, len & 15 as size_t, align);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32(
    mut input: *const ::core::ffi::c_void,
    mut len: size_t,
    mut seed: XXH32_hash_t,
) -> XXH32_hash_t {
    return XXH32_endian_align(input as *const xxh_u8, len, seed as xxh_u32, XXH_unaligned)
        as XXH32_hash_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_createState() -> *mut XXH32_state_t {
    return XXH_malloc(::core::mem::size_of::<XXH32_state_t>() as size_t) as *mut XXH32_state_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_freeState(mut statePtr: *mut XXH32_state_t) -> XXH_errorcode {
    XXH_free(statePtr as *mut ::core::ffi::c_void);
    return XXH_OK;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_copyState(
    mut dstState: *mut XXH32_state_t,
    mut srcState: *const XXH32_state_t,
) {
    XXH_memcpy(
        dstState as *mut ::core::ffi::c_void,
        srcState as *const ::core::ffi::c_void,
        ::core::mem::size_of::<XXH32_state_t>() as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_reset(
    mut statePtr: *mut XXH32_state_t,
    mut seed: XXH32_hash_t,
) -> XXH_errorcode {
    if statePtr.is_null() {
        unreachable!();
    }
    memset(
        statePtr as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<XXH32_state_t>() as size_t,
    );
    (*statePtr).v[0 as ::core::ffi::c_int as usize] = seed
        .wrapping_add(XXH_PRIME32_1 as XXH32_hash_t)
        .wrapping_add(XXH_PRIME32_2 as XXH32_hash_t);
    (*statePtr).v[1 as ::core::ffi::c_int as usize] =
        seed.wrapping_add(XXH_PRIME32_2 as XXH32_hash_t);
    (*statePtr).v[2 as ::core::ffi::c_int as usize] = seed.wrapping_add(0 as XXH32_hash_t);
    (*statePtr).v[3 as ::core::ffi::c_int as usize] =
        seed.wrapping_sub(XXH_PRIME32_1 as XXH32_hash_t);
    return XXH_OK;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_update(
    mut state: *mut XXH32_state_t,
    mut input: *const ::core::ffi::c_void,
    mut len: size_t,
) -> XXH_errorcode {
    if input.is_null() {
        if !(len == 0 as size_t) {
            unreachable!();
        }
        return XXH_OK;
    }
    let mut p: *const xxh_u8 = input as *const xxh_u8;
    let bEnd: *const xxh_u8 = p.offset(len as isize);
    (*state).total_len_32 = ((*state).total_len_32 as ::core::ffi::c_uint)
        .wrapping_add(len as XXH32_hash_t as ::core::ffi::c_uint)
        as XXH32_hash_t as XXH32_hash_t;
    (*state).large_len = ((*state).large_len as ::core::ffi::c_uint
        | ((len >= 16 as size_t) as ::core::ffi::c_int
            | ((*state).total_len_32 >= 16 as XXH32_hash_t) as ::core::ffi::c_int)
            as XXH32_hash_t as ::core::ffi::c_uint) as XXH32_hash_t;
    if ((*state).memsize as size_t).wrapping_add(len) < 16 as size_t {
        XXH_memcpy(
            (&raw mut (*state).mem32 as *mut XXH32_hash_t as *mut xxh_u8)
                .offset((*state).memsize as isize) as *mut ::core::ffi::c_void,
            input,
            len,
        );
        (*state).memsize = ((*state).memsize as ::core::ffi::c_uint)
            .wrapping_add(len as XXH32_hash_t as ::core::ffi::c_uint)
            as XXH32_hash_t as XXH32_hash_t;
        return XXH_OK;
    }
    if (*state).memsize != 0 {
        XXH_memcpy(
            (&raw mut (*state).mem32 as *mut XXH32_hash_t as *mut xxh_u8)
                .offset((*state).memsize as isize) as *mut ::core::ffi::c_void,
            input,
            (16 as XXH32_hash_t).wrapping_sub((*state).memsize) as size_t,
        );
        let mut p32: *const xxh_u32 = &raw mut (*state).mem32 as *mut XXH32_hash_t;
        (*state).v[0 as ::core::ffi::c_int as usize] = XXH32_round(
            (*state).v[0 as ::core::ffi::c_int as usize],
            XXH_readLE32(p32 as *const ::core::ffi::c_void),
        ) as XXH32_hash_t;
        p32 = p32.offset(1);
        (*state).v[1 as ::core::ffi::c_int as usize] = XXH32_round(
            (*state).v[1 as ::core::ffi::c_int as usize],
            XXH_readLE32(p32 as *const ::core::ffi::c_void),
        ) as XXH32_hash_t;
        p32 = p32.offset(1);
        (*state).v[2 as ::core::ffi::c_int as usize] = XXH32_round(
            (*state).v[2 as ::core::ffi::c_int as usize],
            XXH_readLE32(p32 as *const ::core::ffi::c_void),
        ) as XXH32_hash_t;
        p32 = p32.offset(1);
        (*state).v[3 as ::core::ffi::c_int as usize] = XXH32_round(
            (*state).v[3 as ::core::ffi::c_int as usize],
            XXH_readLE32(p32 as *const ::core::ffi::c_void),
        ) as XXH32_hash_t;
        p = p.offset((16 as XXH32_hash_t).wrapping_sub((*state).memsize) as isize);
        (*state).memsize = 0 as XXH32_hash_t;
    }
    if p <= bEnd.offset(-(16 as ::core::ffi::c_int as isize)) {
        let limit: *const xxh_u8 = bEnd.offset(-(16 as ::core::ffi::c_int as isize));
        loop {
            (*state).v[0 as ::core::ffi::c_int as usize] = XXH32_round(
                (*state).v[0 as ::core::ffi::c_int as usize],
                XXH_readLE32(p as *const ::core::ffi::c_void),
            ) as XXH32_hash_t;
            p = p.offset(4 as ::core::ffi::c_int as isize);
            (*state).v[1 as ::core::ffi::c_int as usize] = XXH32_round(
                (*state).v[1 as ::core::ffi::c_int as usize],
                XXH_readLE32(p as *const ::core::ffi::c_void),
            ) as XXH32_hash_t;
            p = p.offset(4 as ::core::ffi::c_int as isize);
            (*state).v[2 as ::core::ffi::c_int as usize] = XXH32_round(
                (*state).v[2 as ::core::ffi::c_int as usize],
                XXH_readLE32(p as *const ::core::ffi::c_void),
            ) as XXH32_hash_t;
            p = p.offset(4 as ::core::ffi::c_int as isize);
            (*state).v[3 as ::core::ffi::c_int as usize] = XXH32_round(
                (*state).v[3 as ::core::ffi::c_int as usize],
                XXH_readLE32(p as *const ::core::ffi::c_void),
            ) as XXH32_hash_t;
            p = p.offset(4 as ::core::ffi::c_int as isize);
            if !(p <= limit) {
                break;
            }
        }
    }
    if p < bEnd {
        XXH_memcpy(
            &raw mut (*state).mem32 as *mut XXH32_hash_t as *mut ::core::ffi::c_void,
            p as *const ::core::ffi::c_void,
            bEnd.offset_from(p) as ::core::ffi::c_long as size_t,
        );
        (*state).memsize =
            bEnd.offset_from(p) as ::core::ffi::c_long as ::core::ffi::c_uint as XXH32_hash_t;
    }
    return XXH_OK;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_digest(mut state: *const XXH32_state_t) -> XXH32_hash_t {
    let mut h32: xxh_u32 = 0;
    if (*state).large_len != 0 {
        h32 = (*state).v[0 as ::core::ffi::c_int as usize]
            .rotate_left(1 as ::core::ffi::c_int as ::core::ffi::c_uint as u32)
            .wrapping_add(
                (*state).v[1 as ::core::ffi::c_int as usize]
                    .rotate_left(7 as ::core::ffi::c_int as ::core::ffi::c_uint as u32),
            )
            .wrapping_add(
                (*state).v[2 as ::core::ffi::c_int as usize]
                    .rotate_left(12 as ::core::ffi::c_int as ::core::ffi::c_uint as u32),
            )
            .wrapping_add(
                (*state).v[3 as ::core::ffi::c_int as usize]
                    .rotate_left(18 as ::core::ffi::c_int as ::core::ffi::c_uint as u32),
            ) as xxh_u32;
    } else {
        h32 = (*state).v[2 as ::core::ffi::c_int as usize]
            .wrapping_add(XXH_PRIME32_5 as XXH32_hash_t) as xxh_u32;
    }
    h32 = (h32 as ::core::ffi::c_uint).wrapping_add((*state).total_len_32 as ::core::ffi::c_uint)
        as xxh_u32 as xxh_u32;
    return XXH32_finalize(
        h32,
        &raw const (*state).mem32 as *const XXH32_hash_t as *const xxh_u8,
        (*state).memsize as size_t,
        XXH_aligned,
    ) as XXH32_hash_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_canonicalFromHash(
    mut dst: *mut XXH32_canonical_t,
    mut hash: XXH32_hash_t,
) {
    hash = XXH_swap32(hash as xxh_u32) as XXH32_hash_t;
    XXH_memcpy(
        dst as *mut ::core::ffi::c_void,
        &raw mut hash as *const ::core::ffi::c_void,
        ::core::mem::size_of::<XXH32_canonical_t>() as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH32_hashFromCanonical(
    mut src: *const XXH32_canonical_t,
) -> XXH32_hash_t {
    return XXH_readBE32(src as *const ::core::ffi::c_void) as XXH32_hash_t;
}
unsafe extern "C" fn XXH_read64(mut ptr: *const ::core::ffi::c_void) -> xxh_u64 {
    return *(ptr as *const xxh_unalign64);
}
unsafe extern "C" fn XXH_swap64(mut x: xxh_u64) -> xxh_u64 {
    return ((x << 56 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
        & 0xff00000000000000 as ::core::ffi::c_ulonglong
        | (x << 40 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff000000000000 as ::core::ffi::c_ulonglong
        | (x << 24 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff0000000000 as ::core::ffi::c_ulonglong
        | (x << 8 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff00000000 as ::core::ffi::c_ulonglong
        | (x >> 8 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff000000 as ::core::ffi::c_ulonglong
        | (x >> 24 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff0000 as ::core::ffi::c_ulonglong
        | (x >> 40 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff00 as ::core::ffi::c_ulonglong
        | (x >> 56 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
            & 0xff as ::core::ffi::c_ulonglong) as xxh_u64;
}
#[inline(always)]
unsafe extern "C" fn XXH_readLE64(mut ptr: *const ::core::ffi::c_void) -> xxh_u64 {
    return if XXH_CPU_LITTLE_ENDIAN != 0 {
        XXH_read64(ptr)
    } else {
        XXH_swap64(XXH_read64(ptr))
    };
}
unsafe extern "C" fn XXH_readBE64(mut ptr: *const ::core::ffi::c_void) -> xxh_u64 {
    return if XXH_CPU_LITTLE_ENDIAN != 0 {
        XXH_swap64(XXH_read64(ptr))
    } else {
        XXH_read64(ptr)
    };
}
#[inline(always)]
unsafe extern "C" fn XXH_readLE64_align(
    mut ptr: *const ::core::ffi::c_void,
    mut align: XXH_alignment,
) -> xxh_u64 {
    if align as ::core::ffi::c_uint == XXH_unaligned as ::core::ffi::c_int as ::core::ffi::c_uint {
        return XXH_readLE64(ptr);
    } else {
        return if XXH_CPU_LITTLE_ENDIAN != 0 {
            *(ptr as *const xxh_u64)
        } else {
            XXH_swap64(*(ptr as *const xxh_u64))
        };
    };
}
pub const XXH_PRIME64_1: ::core::ffi::c_ulonglong = 0x9e3779b185ebca87 as ::core::ffi::c_ulonglong;
pub const XXH_PRIME64_2: ::core::ffi::c_ulonglong = 0xc2b2ae3d27d4eb4f as ::core::ffi::c_ulonglong;
pub const XXH_PRIME64_3: ::core::ffi::c_ulonglong = 0x165667b19e3779f9 as ::core::ffi::c_ulonglong;
pub const XXH_PRIME64_4: ::core::ffi::c_ulonglong = 0x85ebca77c2b2ae63 as ::core::ffi::c_ulonglong;
pub const XXH_PRIME64_5: ::core::ffi::c_ulonglong = 0x27d4eb2f165667c5 as ::core::ffi::c_ulonglong;
unsafe extern "C" fn XXH64_round(mut acc: xxh_u64, mut input: xxh_u64) -> xxh_u64 {
    acc = (acc as ::core::ffi::c_ulonglong)
        .wrapping_add((input as ::core::ffi::c_ulonglong).wrapping_mul(XXH_PRIME64_2))
        as xxh_u64 as xxh_u64;
    acc = acc.rotate_left(31 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32) as xxh_u64;
    acc = (acc as ::core::ffi::c_ulonglong).wrapping_mul(XXH_PRIME64_1) as xxh_u64 as xxh_u64;
    return acc;
}
unsafe extern "C" fn XXH64_mergeRound(mut acc: xxh_u64, mut val: xxh_u64) -> xxh_u64 {
    val = XXH64_round(0 as xxh_u64, val);
    acc = (acc as ::core::ffi::c_ulong ^ val as ::core::ffi::c_ulong) as xxh_u64;
    acc = (acc as ::core::ffi::c_ulonglong)
        .wrapping_mul(XXH_PRIME64_1)
        .wrapping_add(XXH_PRIME64_4) as xxh_u64;
    return acc;
}
unsafe extern "C" fn XXH64_avalanche(mut hash: xxh_u64) -> xxh_u64 {
    hash = (hash as ::core::ffi::c_ulong
        ^ (hash >> 33 as ::core::ffi::c_int) as ::core::ffi::c_ulong) as xxh_u64;
    hash = (hash as ::core::ffi::c_ulonglong).wrapping_mul(XXH_PRIME64_2) as xxh_u64 as xxh_u64;
    hash = (hash as ::core::ffi::c_ulong
        ^ (hash >> 29 as ::core::ffi::c_int) as ::core::ffi::c_ulong) as xxh_u64;
    hash = (hash as ::core::ffi::c_ulonglong).wrapping_mul(XXH_PRIME64_3) as xxh_u64 as xxh_u64;
    hash = (hash as ::core::ffi::c_ulong
        ^ (hash >> 32 as ::core::ffi::c_int) as ::core::ffi::c_ulong) as xxh_u64;
    return hash;
}
unsafe extern "C" fn XXH64_finalize(
    mut hash: xxh_u64,
    mut ptr: *const xxh_u8,
    mut len: size_t,
    mut align: XXH_alignment,
) -> xxh_u64 {
    if ptr.is_null() {
        if !(len == 0 as size_t) {
            unreachable!();
        }
    }
    len = (len as ::core::ffi::c_ulong & 31 as ::core::ffi::c_ulong) as size_t;
    while len >= 8 as size_t {
        let k1: xxh_u64 = XXH64_round(
            0 as xxh_u64,
            XXH_readLE64_align(ptr as *const ::core::ffi::c_void, align),
        ) as xxh_u64;
        ptr = ptr.offset(8 as ::core::ffi::c_int as isize);
        hash = (hash as ::core::ffi::c_ulong ^ k1 as ::core::ffi::c_ulong) as xxh_u64;
        hash = (hash.rotate_left(27 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32)
            as ::core::ffi::c_ulonglong)
            .wrapping_mul(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_4) as xxh_u64;
        len = (len as ::core::ffi::c_ulong).wrapping_sub(8 as ::core::ffi::c_ulong) as size_t
            as size_t;
    }
    if len >= 4 as size_t {
        hash = (hash as ::core::ffi::c_ulonglong
            ^ (XXH_readLE32_align(ptr as *const ::core::ffi::c_void, align) as xxh_u64
                as ::core::ffi::c_ulonglong)
                .wrapping_mul(XXH_PRIME64_1)) as xxh_u64;
        ptr = ptr.offset(4 as ::core::ffi::c_int as isize);
        hash = (hash.rotate_left(23 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32)
            as ::core::ffi::c_ulonglong)
            .wrapping_mul(XXH_PRIME64_2)
            .wrapping_add(XXH_PRIME64_3) as xxh_u64;
        len = (len as ::core::ffi::c_ulong).wrapping_sub(4 as ::core::ffi::c_ulong) as size_t
            as size_t;
    }
    while len > 0 as size_t {
        let fresh7 = ptr;
        ptr = ptr.offset(1);
        hash = (hash as ::core::ffi::c_ulonglong
            ^ (*fresh7 as ::core::ffi::c_ulonglong).wrapping_mul(XXH_PRIME64_5))
            as xxh_u64;
        hash = (hash.rotate_left(11 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32)
            as ::core::ffi::c_ulonglong)
            .wrapping_mul(XXH_PRIME64_1) as xxh_u64;
        len = len.wrapping_sub(1);
    }
    return XXH64_avalanche(hash);
}
#[inline(always)]
unsafe extern "C" fn XXH64_endian_align(
    mut input: *const xxh_u8,
    mut len: size_t,
    mut seed: xxh_u64,
    mut align: XXH_alignment,
) -> xxh_u64 {
    let mut h64: xxh_u64 = 0;
    if input.is_null() {
        if !(len == 0 as size_t) {
            unreachable!();
        }
    }
    if len >= 32 as size_t {
        let bEnd: *const xxh_u8 = input.offset(len as isize);
        let limit: *const xxh_u8 = bEnd.offset(-(31 as ::core::ffi::c_int as isize));
        let mut v1: xxh_u64 = (seed as ::core::ffi::c_ulonglong)
            .wrapping_add(XXH_PRIME64_1)
            .wrapping_add(XXH_PRIME64_2) as xxh_u64;
        let mut v2: xxh_u64 =
            (seed as ::core::ffi::c_ulonglong).wrapping_add(XXH_PRIME64_2) as xxh_u64;
        let mut v3: xxh_u64 = seed.wrapping_add(0 as xxh_u64);
        let mut v4: xxh_u64 =
            (seed as ::core::ffi::c_ulonglong).wrapping_sub(XXH_PRIME64_1) as xxh_u64;
        loop {
            v1 = XXH64_round(
                v1,
                XXH_readLE64_align(input as *const ::core::ffi::c_void, align),
            );
            input = input.offset(8 as ::core::ffi::c_int as isize);
            v2 = XXH64_round(
                v2,
                XXH_readLE64_align(input as *const ::core::ffi::c_void, align),
            );
            input = input.offset(8 as ::core::ffi::c_int as isize);
            v3 = XXH64_round(
                v3,
                XXH_readLE64_align(input as *const ::core::ffi::c_void, align),
            );
            input = input.offset(8 as ::core::ffi::c_int as isize);
            v4 = XXH64_round(
                v4,
                XXH_readLE64_align(input as *const ::core::ffi::c_void, align),
            );
            input = input.offset(8 as ::core::ffi::c_int as isize);
            if !(input < limit) {
                break;
            }
        }
        h64 = v1
            .rotate_left(1 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32)
            .wrapping_add(v2.rotate_left(7 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32))
            .wrapping_add(v3.rotate_left(12 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32))
            .wrapping_add(v4.rotate_left(18 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32))
            as xxh_u64;
        h64 = XXH64_mergeRound(h64, v1);
        h64 = XXH64_mergeRound(h64, v2);
        h64 = XXH64_mergeRound(h64, v3);
        h64 = XXH64_mergeRound(h64, v4);
    } else {
        h64 = (seed as ::core::ffi::c_ulonglong).wrapping_add(XXH_PRIME64_5) as xxh_u64;
    }
    h64 = (h64 as ::core::ffi::c_ulong).wrapping_add(len as xxh_u64 as ::core::ffi::c_ulong)
        as xxh_u64 as xxh_u64;
    return XXH64_finalize(h64, input, len, align);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64(
    mut input: *const ::core::ffi::c_void,
    mut len: size_t,
    mut seed: XXH64_hash_t,
) -> XXH64_hash_t {
    return XXH64_endian_align(input as *const xxh_u8, len, seed as xxh_u64, XXH_unaligned)
        as XXH64_hash_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_createState() -> *mut XXH64_state_t {
    return XXH_malloc(::core::mem::size_of::<XXH64_state_t>() as size_t) as *mut XXH64_state_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_freeState(mut statePtr: *mut XXH64_state_t) -> XXH_errorcode {
    XXH_free(statePtr as *mut ::core::ffi::c_void);
    return XXH_OK;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_copyState(
    mut dstState: *mut XXH64_state_t,
    mut srcState: *const XXH64_state_t,
) {
    XXH_memcpy(
        dstState as *mut ::core::ffi::c_void,
        srcState as *const ::core::ffi::c_void,
        ::core::mem::size_of::<XXH64_state_t>() as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_reset(
    mut statePtr: *mut XXH64_state_t,
    mut seed: XXH64_hash_t,
) -> XXH_errorcode {
    if statePtr.is_null() {
        unreachable!();
    }
    memset(
        statePtr as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<XXH64_state_t>() as size_t,
    );
    (*statePtr).v[0 as ::core::ffi::c_int as usize] = (seed as ::core::ffi::c_ulonglong)
        .wrapping_add(XXH_PRIME64_1)
        .wrapping_add(XXH_PRIME64_2)
        as XXH64_hash_t;
    (*statePtr).v[1 as ::core::ffi::c_int as usize] =
        (seed as ::core::ffi::c_ulonglong).wrapping_add(XXH_PRIME64_2) as XXH64_hash_t;
    (*statePtr).v[2 as ::core::ffi::c_int as usize] = seed.wrapping_add(0 as XXH64_hash_t);
    (*statePtr).v[3 as ::core::ffi::c_int as usize] =
        (seed as ::core::ffi::c_ulonglong).wrapping_sub(XXH_PRIME64_1) as XXH64_hash_t;
    return XXH_OK;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_update(
    mut state: *mut XXH64_state_t,
    mut input: *const ::core::ffi::c_void,
    mut len: size_t,
) -> XXH_errorcode {
    if input.is_null() {
        if !(len == 0 as size_t) {
            unreachable!();
        }
        return XXH_OK;
    }
    let mut p: *const xxh_u8 = input as *const xxh_u8;
    let bEnd: *const xxh_u8 = p.offset(len as isize);
    (*state).total_len = ((*state).total_len as ::core::ffi::c_ulong)
        .wrapping_add(len as ::core::ffi::c_ulong) as XXH64_hash_t
        as XXH64_hash_t;
    if ((*state).memsize as size_t).wrapping_add(len) < 32 as size_t {
        XXH_memcpy(
            (&raw mut (*state).mem64 as *mut XXH64_hash_t as *mut xxh_u8)
                .offset((*state).memsize as isize) as *mut ::core::ffi::c_void,
            input,
            len,
        );
        (*state).memsize = ((*state).memsize as ::core::ffi::c_uint)
            .wrapping_add(len as xxh_u32 as ::core::ffi::c_uint)
            as XXH32_hash_t as XXH32_hash_t;
        return XXH_OK;
    }
    if (*state).memsize != 0 {
        XXH_memcpy(
            (&raw mut (*state).mem64 as *mut XXH64_hash_t as *mut xxh_u8)
                .offset((*state).memsize as isize) as *mut ::core::ffi::c_void,
            input,
            (32 as XXH32_hash_t).wrapping_sub((*state).memsize) as size_t,
        );
        (*state).v[0 as ::core::ffi::c_int as usize] = XXH64_round(
            (*state).v[0 as ::core::ffi::c_int as usize],
            XXH_readLE64(
                (&raw mut (*state).mem64 as *mut XXH64_hash_t)
                    .offset(0 as ::core::ffi::c_int as isize)
                    as *const ::core::ffi::c_void,
            ),
        ) as XXH64_hash_t;
        (*state).v[1 as ::core::ffi::c_int as usize] = XXH64_round(
            (*state).v[1 as ::core::ffi::c_int as usize],
            XXH_readLE64(
                (&raw mut (*state).mem64 as *mut XXH64_hash_t)
                    .offset(1 as ::core::ffi::c_int as isize)
                    as *const ::core::ffi::c_void,
            ),
        ) as XXH64_hash_t;
        (*state).v[2 as ::core::ffi::c_int as usize] = XXH64_round(
            (*state).v[2 as ::core::ffi::c_int as usize],
            XXH_readLE64(
                (&raw mut (*state).mem64 as *mut XXH64_hash_t)
                    .offset(2 as ::core::ffi::c_int as isize)
                    as *const ::core::ffi::c_void,
            ),
        ) as XXH64_hash_t;
        (*state).v[3 as ::core::ffi::c_int as usize] = XXH64_round(
            (*state).v[3 as ::core::ffi::c_int as usize],
            XXH_readLE64(
                (&raw mut (*state).mem64 as *mut XXH64_hash_t)
                    .offset(3 as ::core::ffi::c_int as isize)
                    as *const ::core::ffi::c_void,
            ),
        ) as XXH64_hash_t;
        p = p.offset((32 as XXH32_hash_t).wrapping_sub((*state).memsize) as isize);
        (*state).memsize = 0 as XXH32_hash_t;
    }
    if p.offset(32 as ::core::ffi::c_int as isize) <= bEnd {
        let limit: *const xxh_u8 = bEnd.offset(-(32 as ::core::ffi::c_int as isize));
        loop {
            (*state).v[0 as ::core::ffi::c_int as usize] = XXH64_round(
                (*state).v[0 as ::core::ffi::c_int as usize],
                XXH_readLE64(p as *const ::core::ffi::c_void),
            ) as XXH64_hash_t;
            p = p.offset(8 as ::core::ffi::c_int as isize);
            (*state).v[1 as ::core::ffi::c_int as usize] = XXH64_round(
                (*state).v[1 as ::core::ffi::c_int as usize],
                XXH_readLE64(p as *const ::core::ffi::c_void),
            ) as XXH64_hash_t;
            p = p.offset(8 as ::core::ffi::c_int as isize);
            (*state).v[2 as ::core::ffi::c_int as usize] = XXH64_round(
                (*state).v[2 as ::core::ffi::c_int as usize],
                XXH_readLE64(p as *const ::core::ffi::c_void),
            ) as XXH64_hash_t;
            p = p.offset(8 as ::core::ffi::c_int as isize);
            (*state).v[3 as ::core::ffi::c_int as usize] = XXH64_round(
                (*state).v[3 as ::core::ffi::c_int as usize],
                XXH_readLE64(p as *const ::core::ffi::c_void),
            ) as XXH64_hash_t;
            p = p.offset(8 as ::core::ffi::c_int as isize);
            if !(p <= limit) {
                break;
            }
        }
    }
    if p < bEnd {
        XXH_memcpy(
            &raw mut (*state).mem64 as *mut XXH64_hash_t as *mut ::core::ffi::c_void,
            p as *const ::core::ffi::c_void,
            bEnd.offset_from(p) as ::core::ffi::c_long as size_t,
        );
        (*state).memsize =
            bEnd.offset_from(p) as ::core::ffi::c_long as ::core::ffi::c_uint as XXH32_hash_t;
    }
    return XXH_OK;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_digest(mut state: *const XXH64_state_t) -> XXH64_hash_t {
    let mut h64: xxh_u64 = 0;
    if (*state).total_len >= 32 as XXH64_hash_t {
        h64 = (*state).v[0 as ::core::ffi::c_int as usize]
            .rotate_left(1 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32)
            .wrapping_add(
                (*state).v[1 as ::core::ffi::c_int as usize]
                    .rotate_left(7 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32),
            )
            .wrapping_add(
                (*state).v[2 as ::core::ffi::c_int as usize]
                    .rotate_left(12 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32),
            )
            .wrapping_add(
                (*state).v[3 as ::core::ffi::c_int as usize]
                    .rotate_left(18 as ::core::ffi::c_int as ::core::ffi::c_ulong as u32),
            ) as xxh_u64;
        h64 = XXH64_mergeRound(h64, (*state).v[0 as ::core::ffi::c_int as usize]);
        h64 = XXH64_mergeRound(h64, (*state).v[1 as ::core::ffi::c_int as usize]);
        h64 = XXH64_mergeRound(h64, (*state).v[2 as ::core::ffi::c_int as usize]);
        h64 = XXH64_mergeRound(h64, (*state).v[3 as ::core::ffi::c_int as usize]);
    } else {
        h64 = ((*state).v[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_ulonglong)
            .wrapping_add(XXH_PRIME64_5) as xxh_u64;
    }
    h64 = (h64 as ::core::ffi::c_ulong).wrapping_add((*state).total_len as ::core::ffi::c_ulong)
        as xxh_u64 as xxh_u64;
    return XXH64_finalize(
        h64,
        &raw const (*state).mem64 as *const XXH64_hash_t as *const xxh_u8,
        (*state).total_len as size_t,
        XXH_aligned,
    ) as XXH64_hash_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_canonicalFromHash(
    mut dst: *mut XXH64_canonical_t,
    mut hash: XXH64_hash_t,
) {
    hash = XXH_swap64(hash as xxh_u64) as XXH64_hash_t;
    XXH_memcpy(
        dst as *mut ::core::ffi::c_void,
        &raw mut hash as *const ::core::ffi::c_void,
        ::core::mem::size_of::<XXH64_canonical_t>() as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_XXH64_hashFromCanonical(
    mut src: *const XXH64_canonical_t,
) -> XXH64_hash_t {
    return XXH_readBE64(src as *const ::core::ffi::c_void) as XXH64_hash_t;
}
