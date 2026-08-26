// Static tables from pcre2_compile.c. Auto-generated; do not edit.
#![allow(non_upper_case_globals, dead_code)]
use core::ffi::{c_char, c_int};

#[repr(C)]
pub(crate) struct verbitem {
    pub len: core::ffi::c_uint,
    pub meta: u32,
    pub has_arg: c_int,
}

#[repr(C)]
pub(crate) struct alasitem {
    pub len: core::ffi::c_uint,
    pub meta: u32,
}

#[repr(C)]
pub(crate) struct pso {
    pub name: *const c_char,
    pub length: u16,
    pub type_: u16,
    pub value: u32,
}
unsafe impl Sync for pso {}

#[inline(always)]
pub(crate) fn IS_DIGIT(x: u32) -> bool { x >= crate::internal::CHAR_0 && x <= crate::internal::CHAR_9 }
#[inline(always)]
pub(crate) fn XDIGIT(c: u32) -> u32 { xdigitab[c as usize] as u32 }
#[inline(always)]
pub(crate) fn UPPER_CASE(c: u32) -> u32 { c.wrapping_sub(32) }


pub(crate) const MAX_GROUP_NUMBER: u32 = 65535u32;
pub(crate) const MAX_REPEAT_COUNT: u32 = 65535u32;
pub(crate) const REPEAT_UNLIMITED: u32 = 65536u32;
pub(crate) const COMPILE_WORK_SIZE: usize = 6000;
pub(crate) const C16_WORK_SIZE: usize = 3000;
pub(crate) const GROUPINFO_DEFAULT_SIZE: usize = 256;
pub(crate) const WORK_SIZE_SAFETY_MARGIN: usize = 100;
pub(crate) const NAMED_GROUP_LIST_SIZE: usize = 20;
pub(crate) const PARSED_PATTERN_DEFAULT_SIZE: usize = 1024;
pub(crate) const OFLOW_MAX: c_int = 2147483627;
pub(crate) const ESCAPES_FIRST: u32 = 48;
pub(crate) const ESCAPES_LAST: u32 = 122;
pub(crate) const REQ_UNSET: u32 = 0xffffffffu32;
pub(crate) const REQ_NONE: u32 = 0xfffffffeu32;
pub(crate) const REQ_CASELESS: u32 = 0x00000001u32;
pub(crate) const REQ_VARY: u32 = 0x00000002u32;
pub(crate) const GI_SET_FIXED_LENGTH: u32 = 0x80000000u32;
pub(crate) const GI_NOT_FIXED_LENGTH: u32 = 0x40000000u32;
pub(crate) const GI_FIXED_LENGTH_MASK: u32 = 0x0000ffffu32;
pub(crate) const PSKIP_ALT: u32 = 0;
pub(crate) const PSKIP_CLASS: u32 = 1;
pub(crate) const PSKIP_KET: u32 = 2;
pub(crate) const PSO_OPT: u16 = 0;
pub(crate) const PSO_XOPT: u16 = 1;
pub(crate) const PSO_FLG: u16 = 2;
pub(crate) const PSO_NL: u16 = 3;
pub(crate) const PSO_BSR: u16 = 4;
pub(crate) const PSO_LIMH: u16 = 5;
pub(crate) const PSO_LIMM: u16 = 6;
pub(crate) const PSO_LIMD: u16 = 7;
pub(crate) const PSO_OPTMZ: u16 = 8;
pub(crate) const PUBLIC_LITERAL_COMPILE_OPTIONS: u32 = 0xe689010cu32;
pub(crate) const PUBLIC_COMPILE_OPTIONS: u32 = 0xefffffffu32;
pub(crate) const PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS: u32 = 0x1008cu32;
pub(crate) const PUBLIC_COMPILE_EXTRA_OPTIONS: u32 = 0x1ffffu32;

pub(crate) static meta_extra_lengths: [u8; 73] = [
    0,0,0,0,3,1,3,5,0,0,0,0,0,0,0,0,
    2,3,3,3,3,3,2,0,1,1,0,0,0,0,0,2,
    1,1,0,0,2,3,0,0,0,2,2,0,2,1,0,0,
    0,1,0,1,0,1,0,1,0,0,0,0,0,0,0,0,
    0,2,2,2,0,0,0,0,0,
];

pub(crate) static xdigitab: [u8; 256] = [
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,0x09,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0x0a,0x0b,0x0c,0x0d,0x0e,0x0f,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0x0a,0x0b,0x0c,0x0d,0x0e,0x0f,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
    0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
];

pub(crate) static escapes: [i16; 75] = [
    0,0,0,0,0,0,0,0,0,0,58,59,
    60,61,62,63,64,-1,-4,-14,-6,-25,0,-2,
    -18,0,0,-3,0,0,-12,0,-15,-26,-17,-8,
    0,0,-20,-10,-22,0,-23,91,92,93,94,95,
    96,7,-5,0,-7,27,12,0,-19,0,0,-28,
    0,0,10,0,-16,0,13,-9,9,0,-21,-11,
    0,0,-24,
];

pub(crate) static verbnames: [u8; 43] = [
    0,77,65,82,75,0,65,67,67,69,80,84,0,70,0,70,
    65,73,76,0,67,79,77,77,73,84,0,80,82,85,78,69,
    0,83,75,73,80,0,84,72,69,78,0,
];

pub(crate) static verbs: [verbitem; 9] = [
    verbitem{len:0,meta:0x802d0000,has_arg:1},
    verbitem{len:4,meta:0x802d0000,has_arg:1},
    verbitem{len:6,meta:0x802e0000,has_arg:-1},
    verbitem{len:1,meta:0x802f0000,has_arg:-1},
    verbitem{len:4,meta:0x802f0000,has_arg:-1},
    verbitem{len:6,meta:0x80300000,has_arg:0},
    verbitem{len:5,meta:0x80320000,has_arg:0},
    verbitem{len:4,meta:0x80340000,has_arg:0},
    verbitem{len:4,meta:0x80360000,has_arg:0},
];
pub(crate) const verbcount: c_int = 9;

pub(crate) static verbops: [u32; 11] = [156,166,165,163,164,157,158,159,160,161,162,];

pub(crate) static alasnames: [u8; 229] = [
    112,108,97,0,112,108,98,0,110,97,112,108,97,0,110,97,
    112,108,98,0,110,108,97,0,110,108,98,0,112,111,115,105,
    116,105,118,101,95,108,111,111,107,97,104,101,97,100,0,112,
    111,115,105,116,105,118,101,95,108,111,111,107,98,101,104,105,
    110,100,0,110,111,110,95,97,116,111,109,105,99,95,112,111,
    115,105,116,105,118,101,95,108,111,111,107,97,104,101,97,100,
    0,110,111,110,95,97,116,111,109,105,99,95,112,111,115,105,
    116,105,118,101,95,108,111,111,107,98,101,104,105,110,100,0,
    110,101,103,97,116,105,118,101,95,108,111,111,107,97,104,101,
    97,100,0,110,101,103,97,116,105,118,101,95,108,111,111,107,
    98,101,104,105,110,100,0,115,99,115,0,115,99,97,110,95,
    115,117,98,115,116,114,105,110,103,0,97,116,111,109,105,99,
    0,115,114,0,97,115,114,0,115,99,114,105,112,116,95,114,
    117,110,0,97,116,111,109,105,99,95,115,99,114,105,112,116,
    95,114,117,110,0,
];

pub(crate) static alasmeta: [alasitem; 19] = [
    alasitem{len:3,meta:0x80270000},
    alasitem{len:3,meta:0x80290000},
    alasitem{len:5,meta:0x802b0000},
    alasitem{len:5,meta:0x802c0000},
    alasitem{len:3,meta:0x80280000},
    alasitem{len:3,meta:0x802a0000},
    alasitem{len:18,meta:0x80270000},
    alasitem{len:19,meta:0x80290000},
    alasitem{len:29,meta:0x802b0000},
    alasitem{len:30,meta:0x802c0000},
    alasitem{len:18,meta:0x80280000},
    alasitem{len:19,meta:0x802a0000},
    alasitem{len:3,meta:0x80170000},
    alasitem{len:14,meta:0x80170000},
    alasitem{len:6,meta:0x80020000},
    alasitem{len:2,meta:0x80260000},
    alasitem{len:3,meta:0x8fff0000},
    alasitem{len:10,meta:0x80260000},
    alasitem{len:17,meta:0x8fff0000},
];
pub(crate) const alascount: c_int = 19;

pub(crate) static chartypeoffset: [u32; 4] = [0,13,26,39,];

pub(crate) static posix_names: [u8; 84] = [
    97,108,112,104,97,0,108,111,119,101,114,0,117,112,112,101,
    114,0,97,108,110,117,109,0,97,115,99,105,105,0,98,108,
    97,110,107,0,99,110,116,114,108,0,100,105,103,105,116,0,
    103,114,97,112,104,0,112,114,105,110,116,0,112,117,110,99,
    116,0,115,112,97,99,101,0,119,111,114,100,0,120,100,105,
    103,105,116,0,
];

pub(crate) static posix_name_lengths: [u8; 15] = [5,5,5,5,5,5,5,5,5,5,5,5,4,6,0,];

pub(crate) static posix_substitutes: [c_int; 28] = [
    1,1,2,5,2,9,5,0,-1,0,
    -1,1,2,0,2,13,14,0,15,0,
    16,0,7,0,8,0,17,0,
];

static PSO_NAME_0: [u8; 6] = [85,84,70,56,41,0,];
static PSO_NAME_1: [u8; 5] = [85,84,70,41,0,];
static PSO_NAME_2: [u8; 5] = [85,67,80,41,0,];
static PSO_NAME_3: [u8; 10] = [78,79,84,69,77,80,84,89,41,0,];
static PSO_NAME_4: [u8; 18] = [78,79,84,69,77,80,84,89,95,65,84,83,84,65,82,84,41,0,];
static PSO_NAME_5: [u8; 17] = [78,79,95,65,85,84,79,95,80,79,83,83,69,83,83,41,0,];
static PSO_NAME_6: [u8; 19] = [78,79,95,68,79,84,83,84,65,82,95,65,78,67,72,79,82,41,0,];
static PSO_NAME_7: [u8; 8] = [78,79,95,74,73,84,41,0,];
static PSO_NAME_8: [u8; 14] = [78,79,95,83,84,65,82,84,95,79,80,84,41,0,];
static PSO_NAME_9: [u8; 19] = [67,65,83,69,76,69,83,83,95,82,69,83,84,82,73,67,84,41,0,];
static PSO_NAME_10: [u8; 16] = [84,85,82,75,73,83,72,95,67,65,83,73,78,71,41,0,];
static PSO_NAME_11: [u8; 12] = [76,73,77,73,84,95,72,69,65,80,61,0,];
static PSO_NAME_12: [u8; 13] = [76,73,77,73,84,95,77,65,84,67,72,61,0,];
static PSO_NAME_13: [u8; 13] = [76,73,77,73,84,95,68,69,80,84,72,61,0,];
static PSO_NAME_14: [u8; 17] = [76,73,77,73,84,95,82,69,67,85,82,83,73,79,78,61,0,];
static PSO_NAME_15: [u8; 4] = [67,82,41,0,];
static PSO_NAME_16: [u8; 4] = [76,70,41,0,];
static PSO_NAME_17: [u8; 6] = [67,82,76,70,41,0,];
static PSO_NAME_18: [u8; 5] = [65,78,89,41,0,];
static PSO_NAME_19: [u8; 5] = [78,85,76,41,0,];
static PSO_NAME_20: [u8; 9] = [65,78,89,67,82,76,70,41,0,];
static PSO_NAME_21: [u8; 13] = [66,83,82,95,65,78,89,67,82,76,70,41,0,];
static PSO_NAME_22: [u8; 13] = [66,83,82,95,85,78,73,67,79,68,69,41,0,];
pub(crate) static pso_list: [pso; 23] = [
    pso{name:PSO_NAME_0.as_ptr() as *const c_char,length:5,type_:0,value:0x80000u32},
    pso{name:PSO_NAME_1.as_ptr() as *const c_char,length:4,type_:0,value:0x80000u32},
    pso{name:PSO_NAME_2.as_ptr() as *const c_char,length:4,type_:0,value:0x20000u32},
    pso{name:PSO_NAME_3.as_ptr() as *const c_char,length:9,type_:2,value:0x10000u32},
    pso{name:PSO_NAME_4.as_ptr() as *const c_char,length:17,type_:2,value:0x20000u32},
    pso{name:PSO_NAME_5.as_ptr() as *const c_char,length:16,type_:8,value:0x1u32},
    pso{name:PSO_NAME_6.as_ptr() as *const c_char,length:18,type_:8,value:0x2u32},
    pso{name:PSO_NAME_7.as_ptr() as *const c_char,length:7,type_:2,value:0x80000u32},
    pso{name:PSO_NAME_8.as_ptr() as *const c_char,length:13,type_:8,value:0x4u32},
    pso{name:PSO_NAME_9.as_ptr() as *const c_char,length:18,type_:1,value:0x80u32},
    pso{name:PSO_NAME_10.as_ptr() as *const c_char,length:15,type_:1,value:0x10000u32},
    pso{name:PSO_NAME_11.as_ptr() as *const c_char,length:11,type_:5,value:0x0u32},
    pso{name:PSO_NAME_12.as_ptr() as *const c_char,length:12,type_:6,value:0x0u32},
    pso{name:PSO_NAME_13.as_ptr() as *const c_char,length:12,type_:7,value:0x0u32},
    pso{name:PSO_NAME_14.as_ptr() as *const c_char,length:16,type_:7,value:0x0u32},
    pso{name:PSO_NAME_15.as_ptr() as *const c_char,length:3,type_:3,value:0x1u32},
    pso{name:PSO_NAME_16.as_ptr() as *const c_char,length:3,type_:3,value:0x2u32},
    pso{name:PSO_NAME_17.as_ptr() as *const c_char,length:5,type_:3,value:0x3u32},
    pso{name:PSO_NAME_18.as_ptr() as *const c_char,length:4,type_:3,value:0x4u32},
    pso{name:PSO_NAME_19.as_ptr() as *const c_char,length:4,type_:3,value:0x6u32},
    pso{name:PSO_NAME_20.as_ptr() as *const c_char,length:8,type_:3,value:0x5u32},
    pso{name:PSO_NAME_21.as_ptr() as *const c_char,length:12,type_:4,value:0x2u32},
    pso{name:PSO_NAME_22.as_ptr() as *const c_char,length:12,type_:4,value:0x1u32},
];

pub(crate) static opcode_possessify: [u8; 120] = [
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,42,0,43,0,44,0,45,0,0,0,0,0,0,55,0,
    56,0,57,0,58,0,0,0,0,0,0,68,0,69,0,70,
    0,71,0,0,0,0,0,0,81,0,82,0,83,0,84,0,
    0,0,0,0,0,94,0,95,0,96,0,97,0,0,0,0,
    0,0,106,0,107,0,108,0,109,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,
];

