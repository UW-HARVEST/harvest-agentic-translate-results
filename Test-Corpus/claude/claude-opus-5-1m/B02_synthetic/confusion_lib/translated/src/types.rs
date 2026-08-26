// Rust equivalents of the C aggregate types declared in c_src/src/lib.c.
//
// The layouts below are ABI-identical to what gcc produces on x86-64 SysV
// (verified against the C build):
//
//   PackedFlags  : size 4, align 4
//   TypeConfusion: size 4, align 4
//   ProcessState : size 24, align 8; offsets flags=0 data=4 buffer=8 capacity=16
//
// Bit-field placement inside `PackedFlags` (little-endian, allocated from the
// least significant bit upwards, verified with a layout probe against gcc):
//
//   flag1    -> 0x0000_0001 (bit  0, width  1)
//   flag2    -> 0x0000_0002 (bit  1, width  1)
//   flag3    -> 0x0000_0004 (bit  2, width  1)
//   counter  -> 0x0000_00F8 (bit  3, width  5)
//   mode     -> 0x0000_0700 (bit  8, width  3)
//   status   -> 0x0000_F800 (bit 11, width  5)
//   reserved -> 0xFFFF_0000 (bit 16, width 16)

use core::ffi::{c_char, c_int, c_uint};

/// ```c
/// typedef struct {
///     unsigned int flag1 : 1;
///     unsigned int flag2 : 1;
///     unsigned int flag3 : 1;
///     unsigned int counter : 5;
///     unsigned int mode : 3;
///     unsigned int status : 5;
///     unsigned int reserved : 16;
/// } PackedFlags;
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PackedFlags {
    /// The single 32-bit storage unit holding every bit-field.
    pub bits: c_uint,
}

/// Shift / width for each bit-field of [`PackedFlags`].
const FLAG1: (u32, u32) = (0, 1);
const FLAG2: (u32, u32) = (1, 1);
const FLAG3: (u32, u32) = (2, 1);
const COUNTER: (u32, u32) = (3, 5);
const MODE: (u32, u32) = (8, 3);
const STATUS: (u32, u32) = (11, 5);
const RESERVED: (u32, u32) = (16, 16);

#[inline]
const fn mask((shift, width): (u32, u32)) -> c_uint {
    (((1u64 << width) - 1) as c_uint) << shift
}

#[inline]
const fn get(bits: c_uint, field: (u32, u32)) -> c_uint {
    (bits & mask(field)) >> field.0
}

#[inline]
fn set(bits: &mut c_uint, field: (u32, u32), value: c_uint) {
    let m = mask(field);
    // Storing into a bit-field keeps only the low `width` bits of the value,
    // exactly as a C truncating bit-field assignment does.
    *bits = (*bits & !m) | ((value << field.0) & m);
}

macro_rules! bitfield_accessors {
    ($( $name:ident / $setter:ident => $field:ident ),* $(,)?) => {
        impl PackedFlags {
            $(
                #[inline]
                pub fn $name(&self) -> c_uint {
                    get(self.bits, $field)
                }

                #[inline]
                pub fn $setter(&mut self, value: c_uint) {
                    set(&mut self.bits, $field, value);
                }
            )*
        }
    };
}

bitfield_accessors! {
    flag1 / set_flag1 => FLAG1,
    flag2 / set_flag2 => FLAG2,
    flag3 / set_flag3 => FLAG3,
    counter / set_counter => COUNTER,
    mode / set_mode => MODE,
    status / set_status => STATUS,
    reserved / set_reserved => RESERVED,
}

/// ```c
/// typedef union {
///     int int_val;
///     float float_val;
///     unsigned int uint_val;
///     char bytes[4];
/// } TypeConfusion;
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
pub union TypeConfusion {
    pub int_val: c_int,
    pub float_val: f32,
    pub uint_val: c_uint,
    pub bytes: [c_char; 4],
}

/// ```c
/// typedef struct {
///     PackedFlags flags;
///     TypeConfusion data;
///     char* buffer;
///     int capacity;
/// } ProcessState;
/// ```
#[repr(C)]
pub struct ProcessState {
    pub flags: PackedFlags,
    pub data: TypeConfusion,
    pub buffer: *mut c_char,
    pub capacity: c_int,
}

// Compile-time verification that the layouts match the gcc x86-64 SysV ones.
const _: () = {
    use core::mem::{align_of, offset_of, size_of};

    assert!(size_of::<PackedFlags>() == 4);
    assert!(align_of::<PackedFlags>() == 4);

    assert!(size_of::<TypeConfusion>() == 4);
    assert!(align_of::<TypeConfusion>() == 4);

    assert!(size_of::<ProcessState>() == 24);
    assert!(align_of::<ProcessState>() == 8);
    assert!(offset_of!(ProcessState, flags) == 0);
    assert!(offset_of!(ProcessState, data) == 4);
    assert!(offset_of!(ProcessState, buffer) == 8);
    assert!(offset_of!(ProcessState, capacity) == 16);

    // Bit-field placement inside the storage unit.
    assert!(mask(FLAG1) == 0x0000_0001);
    assert!(mask(FLAG2) == 0x0000_0002);
    assert!(mask(FLAG3) == 0x0000_0004);
    assert!(mask(COUNTER) == 0x0000_00F8);
    assert!(mask(MODE) == 0x0000_0700);
    assert!(mask(STATUS) == 0x0000_F800);
    assert!(mask(RESERVED) == 0xFFFF_0000);
};
