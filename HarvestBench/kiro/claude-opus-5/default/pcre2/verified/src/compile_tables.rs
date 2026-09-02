#![allow(dead_code)]
//! File-scope tables of `pcre2_compile.c`, emitted from the C source.
//! AUTO-GENERATED. Do not edit by hand.

use core::ffi::c_int;

pub const ESCAPES_FIRST: u32 = 48;
pub const ESCAPES_LAST: u32 = 122;

pub static XDIGITAB: [u8; 256] = [
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    0,1,2,3,4,5,6,7,8,9,255,255,255,255,255,255,
    255,10,11,12,13,14,15,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,10,11,12,13,14,15,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
    255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,255,
];

pub static ESCAPES: [i16; 75] = [
    0,0,0,0,0,0,0,0,0,0,58,59,
    60,61,62,63,64,-1,-4,-14,-6,-25,0,-2,
    -18,0,0,-3,0,0,-12,0,-15,-26,-17,-8,
    0,0,-20,-10,-22,0,-23,91,92,93,94,95,
    96,7,-5,0,-7,27,12,0,-19,0,0,-28,
    0,0,10,0,-16,0,13,-9,9,0,-21,-11,
    0,0,-24,
];

pub static VERBNAMES: [u8; 43] = [
    0,77,65,82,75,0,65,67,67,69,80,84,0,70,0,70,
    65,73,76,0,67,79,77,77,73,84,0,80,82,85,78,69,
    0,83,75,73,80,0,84,72,69,78,0,
];

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VerbItem { pub len: core::ffi::c_uint, pub meta: u32, pub has_arg: c_int }

pub static VERBS: [VerbItem; 9] = [
    VerbItem{len:0,meta:0x802d0000,has_arg:1},
    VerbItem{len:4,meta:0x802d0000,has_arg:1},
    VerbItem{len:6,meta:0x802e0000,has_arg:-1},
    VerbItem{len:1,meta:0x802f0000,has_arg:-1},
    VerbItem{len:4,meta:0x802f0000,has_arg:-1},
    VerbItem{len:6,meta:0x80300000,has_arg:0},
    VerbItem{len:5,meta:0x80320000,has_arg:0},
    VerbItem{len:4,meta:0x80340000,has_arg:0},
    VerbItem{len:4,meta:0x80360000,has_arg:0},
];

pub const VERBCOUNT: c_int = 9;

pub static VERBOPS: [u32; 11] = [
    0x9c,0xa6,0xa5,0xa3,0xa4,0x9d,0x9e,0x9f,0xa0,0xa1,
    0xa2,
];

pub static ALASNAMES: [u8; 229] = [
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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AlasItem { pub len: core::ffi::c_uint, pub meta: u32 }

pub static ALASMETA: [AlasItem; 19] = [
    AlasItem{len:3,meta:0x80270000},
    AlasItem{len:3,meta:0x80290000},
    AlasItem{len:5,meta:0x802b0000},
    AlasItem{len:5,meta:0x802c0000},
    AlasItem{len:3,meta:0x80280000},
    AlasItem{len:3,meta:0x802a0000},
    AlasItem{len:18,meta:0x80270000},
    AlasItem{len:19,meta:0x80290000},
    AlasItem{len:29,meta:0x802b0000},
    AlasItem{len:30,meta:0x802c0000},
    AlasItem{len:18,meta:0x80280000},
    AlasItem{len:19,meta:0x802a0000},
    AlasItem{len:3,meta:0x80170000},
    AlasItem{len:14,meta:0x80170000},
    AlasItem{len:6,meta:0x80020000},
    AlasItem{len:2,meta:0x80260000},
    AlasItem{len:3,meta:0x8fff0000},
    AlasItem{len:10,meta:0x80260000},
    AlasItem{len:17,meta:0x8fff0000},
];

pub const ALASCOUNT: c_int = 19;

pub static CHARTYPEOFFSET: [u32; 4] = [
    0x0,0xd,0x1a,0x27,
];

pub static POSIX_NAMES: [u8; 84] = [
    97,108,112,104,97,0,108,111,119,101,114,0,117,112,112,101,
    114,0,97,108,110,117,109,0,97,115,99,105,105,0,98,108,
    97,110,107,0,99,110,116,114,108,0,100,105,103,105,116,0,
    103,114,97,112,104,0,112,114,105,110,116,0,112,117,110,99,
    116,0,115,112,97,99,101,0,119,111,114,100,0,120,100,105,
    103,105,116,0,
];

pub static POSIX_NAME_LENGTHS: [u8; 15] = [
    5,5,5,5,5,5,5,5,5,5,5,5,4,6,0,
];

pub static POSIX_SUBSTITUTES: [c_int; 28] = [
    1,1,2,5,2,9,5,0,
    -1,0,-1,1,2,0,2,13,
    14,0,15,0,16,0,7,0,
    8,0,17,0,
];

pub const PSO_OPT: u16 = 0;
pub const PSO_XOPT: u16 = 1;
pub const PSO_FLG: u16 = 2;
pub const PSO_NL: u16 = 3;
pub const PSO_BSR: u16 = 4;
pub const PSO_LIMH: u16 = 5;
pub const PSO_LIMM: u16 = 6;
pub const PSO_LIMD: u16 = 7;
pub const PSO_OPTMZ: u16 = 8;

#[repr(C)]
pub struct Pso { pub name: &'static [u8], pub length: u16, pub type_: u16, pub value: u32 }

pub static PSO_LIST: [Pso; 23] = [
    Pso{name:b"\x55\x54\x46\x38\x29\0",length:5,type_:0,value:0x80000},
    Pso{name:b"\x55\x54\x46\x29\0",length:4,type_:0,value:0x80000},
    Pso{name:b"\x55\x43\x50\x29\0",length:4,type_:0,value:0x20000},
    Pso{name:b"\x4e\x4f\x54\x45\x4d\x50\x54\x59\x29\0",length:9,type_:2,value:0x10000},
    Pso{name:b"\x4e\x4f\x54\x45\x4d\x50\x54\x59\x5f\x41\x54\x53\x54\x41\x52\x54\x29\0",length:17,type_:2,value:0x20000},
    Pso{name:b"\x4e\x4f\x5f\x41\x55\x54\x4f\x5f\x50\x4f\x53\x53\x45\x53\x53\x29\0",length:16,type_:8,value:0x1},
    Pso{name:b"\x4e\x4f\x5f\x44\x4f\x54\x53\x54\x41\x52\x5f\x41\x4e\x43\x48\x4f\x52\x29\0",length:18,type_:8,value:0x2},
    Pso{name:b"\x4e\x4f\x5f\x4a\x49\x54\x29\0",length:7,type_:2,value:0x80000},
    Pso{name:b"\x4e\x4f\x5f\x53\x54\x41\x52\x54\x5f\x4f\x50\x54\x29\0",length:13,type_:8,value:0x4},
    Pso{name:b"\x43\x41\x53\x45\x4c\x45\x53\x53\x5f\x52\x45\x53\x54\x52\x49\x43\x54\x29\0",length:18,type_:1,value:0x80},
    Pso{name:b"\x54\x55\x52\x4b\x49\x53\x48\x5f\x43\x41\x53\x49\x4e\x47\x29\0",length:15,type_:1,value:0x10000},
    Pso{name:b"\x4c\x49\x4d\x49\x54\x5f\x48\x45\x41\x50\x3d\0",length:11,type_:5,value:0x0},
    Pso{name:b"\x4c\x49\x4d\x49\x54\x5f\x4d\x41\x54\x43\x48\x3d\0",length:12,type_:6,value:0x0},
    Pso{name:b"\x4c\x49\x4d\x49\x54\x5f\x44\x45\x50\x54\x48\x3d\0",length:12,type_:7,value:0x0},
    Pso{name:b"\x4c\x49\x4d\x49\x54\x5f\x52\x45\x43\x55\x52\x53\x49\x4f\x4e\x3d\0",length:16,type_:7,value:0x0},
    Pso{name:b"\x43\x52\x29\0",length:3,type_:3,value:0x1},
    Pso{name:b"\x4c\x46\x29\0",length:3,type_:3,value:0x2},
    Pso{name:b"\x43\x52\x4c\x46\x29\0",length:5,type_:3,value:0x3},
    Pso{name:b"\x41\x4e\x59\x29\0",length:4,type_:3,value:0x4},
    Pso{name:b"\x4e\x55\x4c\x29\0",length:4,type_:3,value:0x6},
    Pso{name:b"\x41\x4e\x59\x43\x52\x4c\x46\x29\0",length:8,type_:3,value:0x5},
    Pso{name:b"\x42\x53\x52\x5f\x41\x4e\x59\x43\x52\x4c\x46\x29\0",length:12,type_:4,value:0x2},
    Pso{name:b"\x42\x53\x52\x5f\x55\x4e\x49\x43\x4f\x44\x45\x29\0",length:12,type_:4,value:0x1},
];

pub static OPCODE_POSSESSIFY: [u8; 120] = [
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,42,0,43,0,44,0,45,0,0,0,0,0,0,55,0,
    56,0,57,0,58,0,0,0,0,0,0,68,0,69,0,70,
    0,71,0,0,0,0,0,0,81,0,82,0,83,0,84,0,
    0,0,0,0,0,94,0,95,0,96,0,97,0,0,0,0,
    0,0,106,0,107,0,108,0,109,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,
];

