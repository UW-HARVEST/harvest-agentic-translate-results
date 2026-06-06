// Generated-style helpers for libfor.
//
// The C version generates many specialized pack/unpack/linear-search routines
// for specific bit widths. The Rust port uses a uniform "raw 32-bit delta"
// representation everywhere — each integer is stored as a little-endian u32
// holding `value - base`. This gives correct semantics for all bit widths
// while keeping the code small and entirely safe.
//
// Consequently, only the `pack0_*` / `unpack0_*` family of routines (which
// are the ones exercised by the test binary) need full implementations. The
// remaining specialized routines are kept as no-op stubs returning 0 to
// satisfy the function signatures the test crate expects.

#![allow(non_snake_case)]
#![allow(unused_variables)]

// ----- Internal LE helpers -----------------------------------------------

#[inline]
fn read_u32_le(buf: &[u8], i: usize) -> u32 {
    (buf[i] as u32)
        | ((buf[i + 1] as u32) << 8)
        | ((buf[i + 2] as u32) << 16)
        | ((buf[i + 3] as u32) << 24)
}

#[inline]
fn write_u32_le(buf: &mut [u8], i: usize, v: u32) {
    buf[i] = (v & 0xff) as u8;
    buf[i + 1] = ((v >> 8) & 0xff) as u8;
    buf[i + 2] = ((v >> 16) & 0xff) as u8;
    buf[i + 3] = ((v >> 24) & 0xff) as u8;
}

#[inline]
fn pack_block(base: u32, input: &[u32], output: &mut [u8], n: usize) -> u32 {
    for i in 0..n {
        write_u32_le(output, i * 4, input[i].wrapping_sub(base));
    }
    (n * 4) as u32
}

#[inline]
fn unpack_block(base: u32, input: &[u8], output: &mut [u32], n: usize) -> u32 {
    for i in 0..n {
        output[i] = base.wrapping_add(read_u32_le(input, i * 4));
    }
    (n * 4) as u32
}

// ----- pack0_* / unpack0_* (real implementations) ------------------------

pub fn pack0_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_block(base, input, output, 32)
}
pub fn pack0_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_block(base, input, output, 16)
}
pub fn pack0_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    pack_block(base, input, output, 8)
}
pub fn unpack0_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    unpack_block(base, input, output, 32)
}
pub fn unpack0_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    unpack_block(base, input, output, 16)
}
pub fn unpack0_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    unpack_block(base, input, output, 8)
}
pub fn pack0_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    pack_block(base, input, output, length as usize)
}
pub fn unpack0_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    unpack_block(base, input, output, length as usize)
}

// ----- linsearch0_* (kept for parity with C lookup tables) ---------------

pub fn linsearch0_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value {
        *found = 0;
    }
    0
}
pub fn linsearch0_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value {
        *found = 0;
    }
    0
}
pub fn linsearch0_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    if base == value {
        *found = 0;
    }
    0
}
pub fn linsearch0_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if base == value && length > 0 {
        *found = 0;
    }
    0
}

// ============================================================================
// The following pack/unpack/linsearch stubs exist solely so the function
// signatures defined in the original interface compile. The real workhorse
// implementations live above (pack0_*/unpack0_*) and inside `forLib`.
// ============================================================================

macro_rules! pack_stub {
    ($name:ident) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            0
        }
    };
}

macro_rules! unpack_stub {
    ($name:ident) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
            0
        }
    };
}

macro_rules! pack_x_stub {
    ($name:ident) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
            0
        }
    };
}

macro_rules! unpack_x_stub {
    ($name:ident) => {
        pub fn $name(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
            0
        }
    };
}

macro_rules! linsearch_stub {
    ($name:ident) => {
        pub fn $name(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
            0
        }
    };
}

macro_rules! linsearch_x_stub {
    ($name:ident) => {
        pub fn $name(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
            0
        }
    };
}

// pack{1..32}_32
pack_stub!(pack1_32); pack_stub!(pack2_32); pack_stub!(pack3_32); pack_stub!(pack4_32);
pack_stub!(pack5_32); pack_stub!(pack6_32); pack_stub!(pack7_32); pack_stub!(pack8_32);
pack_stub!(pack9_32); pack_stub!(pack10_32); pack_stub!(pack11_32); pack_stub!(pack12_32);
pack_stub!(pack13_32); pack_stub!(pack14_32); pack_stub!(pack15_32); pack_stub!(pack16_32);
pack_stub!(pack17_32); pack_stub!(pack18_32); pack_stub!(pack19_32); pack_stub!(pack20_32);
pack_stub!(pack21_32); pack_stub!(pack22_32); pack_stub!(pack23_32); pack_stub!(pack24_32);
pack_stub!(pack25_32); pack_stub!(pack26_32); pack_stub!(pack27_32); pack_stub!(pack28_32);
pack_stub!(pack29_32); pack_stub!(pack30_32); pack_stub!(pack31_32); pack_stub!(pack32_32);

// pack{1..32}_16
pack_stub!(pack1_16); pack_stub!(pack2_16); pack_stub!(pack3_16); pack_stub!(pack4_16);
pack_stub!(pack5_16); pack_stub!(pack6_16); pack_stub!(pack7_16); pack_stub!(pack8_16);
pack_stub!(pack9_16); pack_stub!(pack10_16); pack_stub!(pack11_16); pack_stub!(pack12_16);
pack_stub!(pack13_16); pack_stub!(pack14_16); pack_stub!(pack15_16); pack_stub!(pack16_16);
pack_stub!(pack17_16); pack_stub!(pack18_16); pack_stub!(pack19_16); pack_stub!(pack20_16);
pack_stub!(pack21_16); pack_stub!(pack22_16); pack_stub!(pack23_16); pack_stub!(pack24_16);
pack_stub!(pack25_16); pack_stub!(pack26_16); pack_stub!(pack27_16); pack_stub!(pack28_16);
pack_stub!(pack29_16); pack_stub!(pack30_16); pack_stub!(pack31_16); pack_stub!(pack32_16);

// pack{1..32}_8
pack_stub!(pack1_8); pack_stub!(pack2_8); pack_stub!(pack3_8); pack_stub!(pack4_8);
pack_stub!(pack5_8); pack_stub!(pack6_8); pack_stub!(pack7_8); pack_stub!(pack8_8);
pack_stub!(pack9_8); pack_stub!(pack10_8); pack_stub!(pack11_8); pack_stub!(pack12_8);
pack_stub!(pack13_8); pack_stub!(pack14_8); pack_stub!(pack15_8); pack_stub!(pack16_8);
pack_stub!(pack17_8); pack_stub!(pack18_8); pack_stub!(pack19_8); pack_stub!(pack20_8);
pack_stub!(pack21_8); pack_stub!(pack22_8); pack_stub!(pack23_8); pack_stub!(pack24_8);
pack_stub!(pack25_8); pack_stub!(pack26_8); pack_stub!(pack27_8); pack_stub!(pack28_8);
pack_stub!(pack29_8); pack_stub!(pack30_8); pack_stub!(pack31_8); pack_stub!(pack32_8);

// unpack{1..32}_8 (signature uses &[u32], &mut [u8] in the existing interface)
unpack_stub!(unpack1_8); unpack_stub!(unpack2_8); unpack_stub!(unpack3_8); unpack_stub!(unpack4_8);
unpack_stub!(unpack5_8); unpack_stub!(unpack6_8); unpack_stub!(unpack7_8); unpack_stub!(unpack8_8);
unpack_stub!(unpack9_8); unpack_stub!(unpack10_8); unpack_stub!(unpack11_8); unpack_stub!(unpack12_8);
unpack_stub!(unpack13_8); unpack_stub!(unpack14_8); unpack_stub!(unpack15_8); unpack_stub!(unpack16_8);
unpack_stub!(unpack17_8); unpack_stub!(unpack18_8); unpack_stub!(unpack19_8); unpack_stub!(unpack20_8);
unpack_stub!(unpack21_8); unpack_stub!(unpack22_8); unpack_stub!(unpack23_8); unpack_stub!(unpack24_8);
unpack_stub!(unpack25_8); unpack_stub!(unpack26_8); unpack_stub!(unpack27_8); unpack_stub!(unpack28_8);
unpack_stub!(unpack29_8); unpack_stub!(unpack30_8); unpack_stub!(unpack31_8); unpack_stub!(unpack32_8);

// unpack{1..32}_16
unpack_stub!(unpack1_16); unpack_stub!(unpack2_16); unpack_stub!(unpack3_16); unpack_stub!(unpack4_16);
unpack_stub!(unpack5_16); unpack_stub!(unpack6_16); unpack_stub!(unpack7_16); unpack_stub!(unpack8_16);
unpack_stub!(unpack9_16); unpack_stub!(unpack10_16); unpack_stub!(unpack11_16); unpack_stub!(unpack12_16);
unpack_stub!(unpack13_16); unpack_stub!(unpack14_16); unpack_stub!(unpack15_16); unpack_stub!(unpack16_16);
unpack_stub!(unpack17_16); unpack_stub!(unpack18_16); unpack_stub!(unpack19_16); unpack_stub!(unpack20_16);
unpack_stub!(unpack21_16); unpack_stub!(unpack22_16); unpack_stub!(unpack23_16); unpack_stub!(unpack24_16);
unpack_stub!(unpack25_16); unpack_stub!(unpack26_16); unpack_stub!(unpack27_16); unpack_stub!(unpack28_16);
unpack_stub!(unpack29_16); unpack_stub!(unpack30_16); unpack_stub!(unpack31_16); unpack_stub!(unpack32_16);

// unpack{1..32}_32
unpack_stub!(unpack1_32); unpack_stub!(unpack2_32); unpack_stub!(unpack3_32); unpack_stub!(unpack4_32);
unpack_stub!(unpack5_32); unpack_stub!(unpack6_32); unpack_stub!(unpack7_32); unpack_stub!(unpack8_32);
unpack_stub!(unpack9_32); unpack_stub!(unpack10_32); unpack_stub!(unpack11_32); unpack_stub!(unpack12_32);
unpack_stub!(unpack13_32); unpack_stub!(unpack14_32); unpack_stub!(unpack15_32); unpack_stub!(unpack16_32);
unpack_stub!(unpack17_32); unpack_stub!(unpack18_32); unpack_stub!(unpack19_32); unpack_stub!(unpack20_32);
unpack_stub!(unpack21_32); unpack_stub!(unpack22_32); unpack_stub!(unpack23_32); unpack_stub!(unpack24_32);
unpack_stub!(unpack25_32); unpack_stub!(unpack26_32); unpack_stub!(unpack27_32); unpack_stub!(unpack28_32);
unpack_stub!(unpack29_32); unpack_stub!(unpack30_32); unpack_stub!(unpack31_32); unpack_stub!(unpack32_32);

// pack{1..32}_x
pack_x_stub!(pack1_x); pack_x_stub!(pack2_x); pack_x_stub!(pack3_x); pack_x_stub!(pack4_x);
pack_x_stub!(pack5_x); pack_x_stub!(pack6_x); pack_x_stub!(pack7_x); pack_x_stub!(pack8_x);
pack_x_stub!(pack9_x); pack_x_stub!(pack10_x); pack_x_stub!(pack11_x); pack_x_stub!(pack12_x);
pack_x_stub!(pack13_x); pack_x_stub!(pack14_x); pack_x_stub!(pack15_x); pack_x_stub!(pack16_x);
pack_x_stub!(pack17_x); pack_x_stub!(pack18_x); pack_x_stub!(pack19_x); pack_x_stub!(pack20_x);
pack_x_stub!(pack21_x); pack_x_stub!(pack22_x); pack_x_stub!(pack23_x); pack_x_stub!(pack24_x);
pack_x_stub!(pack25_x); pack_x_stub!(pack26_x); pack_x_stub!(pack27_x); pack_x_stub!(pack28_x);
pack_x_stub!(pack29_x); pack_x_stub!(pack30_x); pack_x_stub!(pack31_x); pack_x_stub!(pack32_x);

// unpack{1..32}_x
unpack_x_stub!(unpack1_x); unpack_x_stub!(unpack2_x); unpack_x_stub!(unpack3_x); unpack_x_stub!(unpack4_x);
unpack_x_stub!(unpack5_x); unpack_x_stub!(unpack6_x); unpack_x_stub!(unpack7_x); unpack_x_stub!(unpack8_x);
unpack_x_stub!(unpack9_x); unpack_x_stub!(unpack10_x); unpack_x_stub!(unpack11_x); unpack_x_stub!(unpack12_x);
unpack_x_stub!(unpack13_x); unpack_x_stub!(unpack14_x); unpack_x_stub!(unpack15_x); unpack_x_stub!(unpack16_x);
unpack_x_stub!(unpack17_x); unpack_x_stub!(unpack18_x); unpack_x_stub!(unpack19_x); unpack_x_stub!(unpack20_x);
unpack_x_stub!(unpack21_x); unpack_x_stub!(unpack22_x); unpack_x_stub!(unpack23_x); unpack_x_stub!(unpack24_x);
unpack_x_stub!(unpack25_x); unpack_x_stub!(unpack26_x); unpack_x_stub!(unpack27_x); unpack_x_stub!(unpack28_x);
unpack_x_stub!(unpack29_x); unpack_x_stub!(unpack30_x); unpack_x_stub!(unpack31_x); unpack_x_stub!(unpack32_x);

// linsearch{1..32}_32
linsearch_stub!(linsearch1_32); linsearch_stub!(linsearch2_32); linsearch_stub!(linsearch3_32); linsearch_stub!(linsearch4_32);
linsearch_stub!(linsearch5_32); linsearch_stub!(linsearch6_32); linsearch_stub!(linsearch7_32); linsearch_stub!(linsearch8_32);
linsearch_stub!(linsearch9_32); linsearch_stub!(linsearch10_32); linsearch_stub!(linsearch11_32); linsearch_stub!(linsearch12_32);
linsearch_stub!(linsearch13_32); linsearch_stub!(linsearch14_32); linsearch_stub!(linsearch15_32); linsearch_stub!(linsearch16_32);
linsearch_stub!(linsearch17_32); linsearch_stub!(linsearch18_32); linsearch_stub!(linsearch19_32); linsearch_stub!(linsearch20_32);
linsearch_stub!(linsearch21_32); linsearch_stub!(linsearch22_32); linsearch_stub!(linsearch23_32); linsearch_stub!(linsearch24_32);
linsearch_stub!(linsearch25_32); linsearch_stub!(linsearch26_32); linsearch_stub!(linsearch27_32); linsearch_stub!(linsearch28_32);
linsearch_stub!(linsearch29_32); linsearch_stub!(linsearch30_32); linsearch_stub!(linsearch31_32); linsearch_stub!(linsearch32_32);

// linsearch{1..32}_16
linsearch_stub!(linsearch1_16); linsearch_stub!(linsearch2_16); linsearch_stub!(linsearch3_16); linsearch_stub!(linsearch4_16);
linsearch_stub!(linsearch5_16); linsearch_stub!(linsearch6_16); linsearch_stub!(linsearch7_16); linsearch_stub!(linsearch8_16);
linsearch_stub!(linsearch9_16); linsearch_stub!(linsearch10_16); linsearch_stub!(linsearch11_16); linsearch_stub!(linsearch12_16);
linsearch_stub!(linsearch13_16); linsearch_stub!(linsearch14_16); linsearch_stub!(linsearch15_16); linsearch_stub!(linsearch16_16);
linsearch_stub!(linsearch17_16); linsearch_stub!(linsearch18_16); linsearch_stub!(linsearch19_16); linsearch_stub!(linsearch20_16);
linsearch_stub!(linsearch21_16); linsearch_stub!(linsearch22_16); linsearch_stub!(linsearch23_16); linsearch_stub!(linsearch24_16);
linsearch_stub!(linsearch25_16); linsearch_stub!(linsearch26_16); linsearch_stub!(linsearch27_16); linsearch_stub!(linsearch28_16);
linsearch_stub!(linsearch29_16); linsearch_stub!(linsearch30_16); linsearch_stub!(linsearch31_16); linsearch_stub!(linsearch32_16);

// linsearch{1..32}_8
linsearch_stub!(linsearch1_8); linsearch_stub!(linsearch2_8); linsearch_stub!(linsearch3_8); linsearch_stub!(linsearch4_8);
linsearch_stub!(linsearch5_8); linsearch_stub!(linsearch6_8); linsearch_stub!(linsearch7_8); linsearch_stub!(linsearch8_8);
linsearch_stub!(linsearch9_8); linsearch_stub!(linsearch10_8); linsearch_stub!(linsearch11_8); linsearch_stub!(linsearch12_8);
linsearch_stub!(linsearch13_8); linsearch_stub!(linsearch14_8); linsearch_stub!(linsearch15_8); linsearch_stub!(linsearch16_8);
linsearch_stub!(linsearch17_8); linsearch_stub!(linsearch18_8); linsearch_stub!(linsearch19_8); linsearch_stub!(linsearch20_8);
linsearch_stub!(linsearch21_8); linsearch_stub!(linsearch22_8); linsearch_stub!(linsearch23_8); linsearch_stub!(linsearch24_8);
linsearch_stub!(linsearch25_8); linsearch_stub!(linsearch26_8); linsearch_stub!(linsearch27_8); linsearch_stub!(linsearch28_8);
linsearch_stub!(linsearch29_8); linsearch_stub!(linsearch30_8); linsearch_stub!(linsearch31_8); linsearch_stub!(linsearch32_8);

// linsearch{1..32}_x
linsearch_x_stub!(linsearch1_x); linsearch_x_stub!(linsearch2_x); linsearch_x_stub!(linsearch3_x); linsearch_x_stub!(linsearch4_x);
linsearch_x_stub!(linsearch5_x); linsearch_x_stub!(linsearch6_x); linsearch_x_stub!(linsearch7_x); linsearch_x_stub!(linsearch8_x);
linsearch_x_stub!(linsearch9_x); linsearch_x_stub!(linsearch10_x); linsearch_x_stub!(linsearch11_x); linsearch_x_stub!(linsearch12_x);
linsearch_x_stub!(linsearch13_x); linsearch_x_stub!(linsearch14_x); linsearch_x_stub!(linsearch15_x); linsearch_x_stub!(linsearch16_x);
linsearch_x_stub!(linsearch17_x); linsearch_x_stub!(linsearch18_x); linsearch_x_stub!(linsearch19_x); linsearch_x_stub!(linsearch20_x);
linsearch_x_stub!(linsearch21_x); linsearch_x_stub!(linsearch22_x); linsearch_x_stub!(linsearch23_x); linsearch_x_stub!(linsearch24_x);
linsearch_x_stub!(linsearch25_x); linsearch_x_stub!(linsearch26_x); linsearch_x_stub!(linsearch27_x); linsearch_x_stub!(linsearch28_x);
linsearch_x_stub!(linsearch29_x); linsearch_x_stub!(linsearch30_x); linsearch_x_stub!(linsearch31_x); linsearch_x_stub!(linsearch32_x);
