pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct bs_t {
    pub buf: *const uint8_t,
    pub pos: ::core::ffi::c_int,
    pub limit: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct L3_gr_info_t {
    pub sfbtab: *const uint8_t,
    pub part_23_length: uint16_t,
    pub big_values: uint16_t,
    pub scalefac_compress: uint16_t,
    pub global_gain: uint8_t,
    pub block_type: uint8_t,
    pub mixed_block_flag: uint8_t,
    pub n_long_sfb: uint8_t,
    pub n_short_sfb: uint8_t,
    pub table_select: [uint8_t; 3],
    pub region_count: [uint8_t; 3],
    pub subblock_gain: [uint8_t; 3],
    pub preflag: uint8_t,
    pub scalefac_scale: uint8_t,
    pub count1_table: uint8_t,
    pub scfsi: uint8_t,
}
unsafe extern "C" fn get_bits(mut bs: *mut bs_t, mut n: ::core::ffi::c_int) -> uint32_t {
    let mut next: uint32_t = 0;
    let mut cache: uint32_t = 0 as uint32_t;
    let mut s: uint32_t = ((*bs).pos & 7 as ::core::ffi::c_int) as uint32_t;
    let mut shl: ::core::ffi::c_int = (n as uint32_t).wrapping_add(s) as ::core::ffi::c_int;
    let mut p: *const uint8_t = (*bs)
        .buf
        .offset(((*bs).pos >> 3 as ::core::ffi::c_int) as isize);
    (*bs).pos += n;
    if (*bs).pos > (*bs).limit {
        return 0 as uint32_t;
    }
    let fresh0 = p;
    p = p.offset(1);
    next = (*fresh0 as ::core::ffi::c_int & 255 as ::core::ffi::c_int >> s) as uint32_t;
    loop {
        shl -= 8 as ::core::ffi::c_int;
        if !(shl > 0 as ::core::ffi::c_int) {
            break;
        }
        cache = (cache as ::core::ffi::c_uint | (next << shl) as ::core::ffi::c_uint) as uint32_t;
        let fresh1 = p;
        p = p.offset(1);
        next = *fresh1 as uint32_t;
    }
    return cache | next >> -shl;
}
#[no_mangle]
pub unsafe extern "C" fn read_side_info(
    mut bs: *mut bs_t,
    mut gr: *mut L3_gr_info_t,
    mut hdr: *const uint8_t,
) -> ::core::ffi::c_int {
    static mut g_scf_long: [[uint8_t; 23]; 8] = [
        [
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            38 as ::core::ffi::c_int as uint8_t,
            46 as ::core::ffi::c_int as uint8_t,
            52 as ::core::ffi::c_int as uint8_t,
            60 as ::core::ffi::c_int as uint8_t,
            68 as ::core::ffi::c_int as uint8_t,
            58 as ::core::ffi::c_int as uint8_t,
            54 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            48 as ::core::ffi::c_int as uint8_t,
            56 as ::core::ffi::c_int as uint8_t,
            64 as ::core::ffi::c_int as uint8_t,
            76 as ::core::ffi::c_int as uint8_t,
            90 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            38 as ::core::ffi::c_int as uint8_t,
            46 as ::core::ffi::c_int as uint8_t,
            52 as ::core::ffi::c_int as uint8_t,
            60 as ::core::ffi::c_int as uint8_t,
            68 as ::core::ffi::c_int as uint8_t,
            58 as ::core::ffi::c_int as uint8_t,
            54 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            22 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            38 as ::core::ffi::c_int as uint8_t,
            46 as ::core::ffi::c_int as uint8_t,
            54 as ::core::ffi::c_int as uint8_t,
            62 as ::core::ffi::c_int as uint8_t,
            70 as ::core::ffi::c_int as uint8_t,
            76 as ::core::ffi::c_int as uint8_t,
            36 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            38 as ::core::ffi::c_int as uint8_t,
            46 as ::core::ffi::c_int as uint8_t,
            52 as ::core::ffi::c_int as uint8_t,
            60 as ::core::ffi::c_int as uint8_t,
            68 as ::core::ffi::c_int as uint8_t,
            58 as ::core::ffi::c_int as uint8_t,
            54 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            34 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            50 as ::core::ffi::c_int as uint8_t,
            54 as ::core::ffi::c_int as uint8_t,
            76 as ::core::ffi::c_int as uint8_t,
            158 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            22 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            34 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            46 as ::core::ffi::c_int as uint8_t,
            54 as ::core::ffi::c_int as uint8_t,
            54 as ::core::ffi::c_int as uint8_t,
            192 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            38 as ::core::ffi::c_int as uint8_t,
            46 as ::core::ffi::c_int as uint8_t,
            56 as ::core::ffi::c_int as uint8_t,
            68 as ::core::ffi::c_int as uint8_t,
            84 as ::core::ffi::c_int as uint8_t,
            102 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
    ];
    static mut g_scf_short: [[uint8_t; 40]; 8] = [
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            36 as ::core::ffi::c_int as uint8_t,
            36 as ::core::ffi::c_int as uint8_t,
            36 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            44 as ::core::ffi::c_int as uint8_t,
            44 as ::core::ffi::c_int as uint8_t,
            44 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            22 as ::core::ffi::c_int as uint8_t,
            22 as ::core::ffi::c_int as uint8_t,
            22 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            56 as ::core::ffi::c_int as uint8_t,
            56 as ::core::ffi::c_int as uint8_t,
            56 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            66 as ::core::ffi::c_int as uint8_t,
            66 as ::core::ffi::c_int as uint8_t,
            66 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            34 as ::core::ffi::c_int as uint8_t,
            34 as ::core::ffi::c_int as uint8_t,
            34 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
    ];
    static mut g_scf_mixed: [[uint8_t; 40]; 8] = [
        [
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
            0,
            0,
            0,
        ],
        [
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            28 as ::core::ffi::c_int as uint8_t,
            36 as ::core::ffi::c_int as uint8_t,
            36 as ::core::ffi::c_int as uint8_t,
            36 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            2 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
        ],
        [
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
            0,
            0,
            0,
        ],
        [
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            32 as ::core::ffi::c_int as uint8_t,
            44 as ::core::ffi::c_int as uint8_t,
            44 as ::core::ffi::c_int as uint8_t,
            44 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
            0,
            0,
            0,
        ],
        [
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            24 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            40 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
            0,
            0,
            0,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            18 as ::core::ffi::c_int as uint8_t,
            22 as ::core::ffi::c_int as uint8_t,
            22 as ::core::ffi::c_int as uint8_t,
            22 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            30 as ::core::ffi::c_int as uint8_t,
            56 as ::core::ffi::c_int as uint8_t,
            56 as ::core::ffi::c_int as uint8_t,
            56 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
            0,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            10 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            14 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            66 as ::core::ffi::c_int as uint8_t,
            66 as ::core::ffi::c_int as uint8_t,
            66 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
            0,
        ],
        [
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            4 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            6 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            8 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            16 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            20 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            26 as ::core::ffi::c_int as uint8_t,
            34 as ::core::ffi::c_int as uint8_t,
            34 as ::core::ffi::c_int as uint8_t,
            34 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            42 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            12 as ::core::ffi::c_int as uint8_t,
            0 as ::core::ffi::c_int as uint8_t,
            0,
        ],
    ];
    let mut tables: ::core::ffi::c_uint = 0;
    let mut scfsi: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut main_data_begin: ::core::ffi::c_int = 0;
    let mut part_23_sum: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut sr_idx: ::core::ffi::c_int = (*hdr.offset(2 as ::core::ffi::c_int as isize)
        as ::core::ffi::c_int
        >> 2 as ::core::ffi::c_int
        & 3 as ::core::ffi::c_int)
        + ((*hdr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            >> 3 as ::core::ffi::c_int
            & 1 as ::core::ffi::c_int)
            + (*hdr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                >> 4 as ::core::ffi::c_int
                & 1 as ::core::ffi::c_int))
            * 3 as ::core::ffi::c_int;
    sr_idx -= (sr_idx != 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
    let mut gr_count: ::core::ffi::c_int = if *hdr.offset(3 as ::core::ffi::c_int as isize)
        as ::core::ffi::c_int
        & 0xc0 as ::core::ffi::c_int
        == 0xc0 as ::core::ffi::c_int
    {
        1 as ::core::ffi::c_int
    } else {
        2 as ::core::ffi::c_int
    };
    if *hdr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        & 0x8 as ::core::ffi::c_int
        != 0
    {
        gr_count *= 2 as ::core::ffi::c_int;
        main_data_begin = get_bits(bs, 9 as ::core::ffi::c_int) as ::core::ffi::c_int;
        scfsi = get_bits(bs, 7 as ::core::ffi::c_int + gr_count) as ::core::ffi::c_uint;
    } else {
        main_data_begin =
            (get_bits(bs, 8 as ::core::ffi::c_int + gr_count) >> gr_count) as ::core::ffi::c_int;
    }
    loop {
        if *hdr.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0xc0 as ::core::ffi::c_int
            == 0xc0 as ::core::ffi::c_int
        {
            scfsi <<= 4 as ::core::ffi::c_int;
        }
        (*gr).part_23_length = get_bits(bs, 12 as ::core::ffi::c_int) as uint16_t;
        part_23_sum += (*gr).part_23_length as ::core::ffi::c_int;
        (*gr).big_values = get_bits(bs, 9 as ::core::ffi::c_int) as uint16_t;
        if (*gr).big_values as ::core::ffi::c_int > 288 as ::core::ffi::c_int {
            return -(1 as ::core::ffi::c_int);
        }
        (*gr).global_gain = get_bits(bs, 8 as ::core::ffi::c_int) as uint8_t;
        (*gr).scalefac_compress = get_bits(
            bs,
            if *hdr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0x8 as ::core::ffi::c_int
                != 0
            {
                4 as ::core::ffi::c_int
            } else {
                9 as ::core::ffi::c_int
            },
        ) as uint16_t;
        (*gr).sfbtab = &raw const *(&raw const g_scf_long as *const [uint8_t; 23])
            .offset(sr_idx as isize) as *const uint8_t;
        (*gr).n_long_sfb = 22 as uint8_t;
        (*gr).n_short_sfb = 0 as uint8_t;
        if get_bits(bs, 1 as ::core::ffi::c_int) != 0 {
            (*gr).block_type = get_bits(bs, 2 as ::core::ffi::c_int) as uint8_t;
            if (*gr).block_type == 0 {
                return -(1 as ::core::ffi::c_int);
            }
            (*gr).mixed_block_flag = get_bits(bs, 1 as ::core::ffi::c_int) as uint8_t;
            (*gr).region_count[0 as ::core::ffi::c_int as usize] = 7 as uint8_t;
            (*gr).region_count[1 as ::core::ffi::c_int as usize] = 255 as uint8_t;
            if (*gr).block_type as ::core::ffi::c_int == 2 as ::core::ffi::c_int {
                scfsi &= 0xf0f as ::core::ffi::c_uint;
                if (*gr).mixed_block_flag == 0 {
                    (*gr).region_count[0 as ::core::ffi::c_int as usize] = 8 as uint8_t;
                    (*gr).sfbtab = &raw const *(&raw const g_scf_short as *const [uint8_t; 40])
                        .offset(sr_idx as isize)
                        as *const uint8_t;
                    (*gr).n_long_sfb = 0 as uint8_t;
                    (*gr).n_short_sfb = 39 as uint8_t;
                } else {
                    (*gr).sfbtab = &raw const *(&raw const g_scf_mixed as *const [uint8_t; 40])
                        .offset(sr_idx as isize)
                        as *const uint8_t;
                    (*gr).n_long_sfb = (if *hdr.offset(1 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        & 0x8 as ::core::ffi::c_int
                        != 0
                    {
                        8 as ::core::ffi::c_int
                    } else {
                        6 as ::core::ffi::c_int
                    }) as uint8_t;
                    (*gr).n_short_sfb = 30 as uint8_t;
                }
            }
            tables = get_bits(bs, 10 as ::core::ffi::c_int) as ::core::ffi::c_uint;
            tables <<= 5 as ::core::ffi::c_int;
            (*gr).subblock_gain[0 as ::core::ffi::c_int as usize] =
                get_bits(bs, 3 as ::core::ffi::c_int) as uint8_t;
            (*gr).subblock_gain[1 as ::core::ffi::c_int as usize] =
                get_bits(bs, 3 as ::core::ffi::c_int) as uint8_t;
            (*gr).subblock_gain[2 as ::core::ffi::c_int as usize] =
                get_bits(bs, 3 as ::core::ffi::c_int) as uint8_t;
        } else {
            (*gr).block_type = 0 as uint8_t;
            (*gr).mixed_block_flag = 0 as uint8_t;
            tables = get_bits(bs, 15 as ::core::ffi::c_int) as ::core::ffi::c_uint;
            (*gr).region_count[0 as ::core::ffi::c_int as usize] =
                get_bits(bs, 4 as ::core::ffi::c_int) as uint8_t;
            (*gr).region_count[1 as ::core::ffi::c_int as usize] =
                get_bits(bs, 3 as ::core::ffi::c_int) as uint8_t;
            (*gr).region_count[2 as ::core::ffi::c_int as usize] = 255 as uint8_t;
        }
        (*gr).table_select[0 as ::core::ffi::c_int as usize] =
            (tables >> 10 as ::core::ffi::c_int) as uint8_t;
        (*gr).table_select[1 as ::core::ffi::c_int as usize] =
            (tables >> 5 as ::core::ffi::c_int & 31 as ::core::ffi::c_uint) as uint8_t;
        (*gr).table_select[2 as ::core::ffi::c_int as usize] =
            (tables & 31 as ::core::ffi::c_uint) as uint8_t;
        (*gr).preflag = (if *hdr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0x8 as ::core::ffi::c_int
            != 0
        {
            get_bits(bs, 1 as ::core::ffi::c_int)
        } else {
            ((*gr).scalefac_compress as ::core::ffi::c_int >= 500 as ::core::ffi::c_int)
                as ::core::ffi::c_int as uint32_t
        }) as uint8_t;
        (*gr).scalefac_scale = get_bits(bs, 1 as ::core::ffi::c_int) as uint8_t;
        (*gr).count1_table = get_bits(bs, 1 as ::core::ffi::c_int) as uint8_t;
        (*gr).scfsi = (scfsi >> 12 as ::core::ffi::c_int & 15 as ::core::ffi::c_uint) as uint8_t;
        scfsi <<= 4 as ::core::ffi::c_int;
        gr = gr.offset(1);
        gr_count -= 1;
        if !(gr_count != 0) {
            break;
        }
    }
    if part_23_sum + (*bs).pos > (*bs).limit + main_data_begin * 8 as ::core::ffi::c_int {
        return -(1 as ::core::ffi::c_int);
    }
    return main_data_begin;
}
