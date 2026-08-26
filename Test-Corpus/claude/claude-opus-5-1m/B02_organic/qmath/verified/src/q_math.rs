//! Translation of `c_src/src/q_math.c` -- "stateless support routines that are
//! included in each code module".
//!
//! Every function is exported with `#[no_mangle] extern "C"` under its original
//! C name so that the shared library built from this crate is a drop-in
//! replacement for the one built from `q_math.c`.
//!
//! ## Faithfulness notes
//!
//! * `vec_t` is `float`, so all vector arithmetic is done in `f32`.  The few
//!   places where the C code mixes in a `double` constant (`M_PI`, `2.0`,
//!   `0.5`, `360.0/65536`, ...) or calls a `<math.h>` function (`sqrt`, `sin`,
//!   `cos`, `atan2`, `fabs`) promote to `f64` exactly like C's usual arithmetic
//!   conversions do, and the result is rounded back to `f32` on assignment.
//! * `(int)` casts of floating point values use [`f32_to_i32`]/[`f64_to_i32`],
//!   which reproduce `cvttss2si`/`cvttsd2si` (`0x80000000` for NaN and for
//!   out-of-range values) instead of Rust's saturating `as` conversion.
//! * Signed integer overflow (`Q_rand`) uses `wrapping_*`, matching gcc's
//!   two's-complement behaviour.
//! * Pointer parameters are kept as raw pointers, including the ones that may
//!   legally be `NULL` (`DirToByte`, `AngleVectors`) and the ones that the C
//!   code happily lets alias (`MakeNormalVectors`), so reads and writes happen
//!   in exactly the same order as in the C source.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use crate::q_shared::*;
use core::ffi::{c_int, c_uint};
use core::ptr::addr_of;

// ---------------------------------------------------------------------------
// globals
// ---------------------------------------------------------------------------

/// `vec3_t vec3_origin = {0,0,0};`
#[no_mangle]
pub static mut vec3_origin: [vec_t; 3] = [0.0, 0.0, 0.0];

/// `vec3_t axisDefault[3] = { { 1, 0, 0 }, { 0, 1, 0 }, { 0, 0, 1 } };`
#[no_mangle]
pub static mut axisDefault: [[vec_t; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// `vec4_t colorBlack = {0, 0, 0, 1};`
#[no_mangle]
pub static mut colorBlack: [vec_t; 4] = [0.0, 0.0, 0.0, 1.0];
/// `vec4_t colorRed = {1, 0, 0, 1};`
#[no_mangle]
pub static mut colorRed: [vec_t; 4] = [1.0, 0.0, 0.0, 1.0];
/// `vec4_t colorGreen = {0, 1, 0, 1};`
#[no_mangle]
pub static mut colorGreen: [vec_t; 4] = [0.0, 1.0, 0.0, 1.0];
/// `vec4_t colorBlue = {0, 0, 1, 1};`
#[no_mangle]
pub static mut colorBlue: [vec_t; 4] = [0.0, 0.0, 1.0, 1.0];
/// `vec4_t colorYellow = {1, 1, 0, 1};`
#[no_mangle]
pub static mut colorYellow: [vec_t; 4] = [1.0, 1.0, 0.0, 1.0];
/// `vec4_t colorMagenta= {1, 0, 1, 1};`
#[no_mangle]
pub static mut colorMagenta: [vec_t; 4] = [1.0, 0.0, 1.0, 1.0];
/// `vec4_t colorCyan = {0, 1, 1, 1};`
#[no_mangle]
pub static mut colorCyan: [vec_t; 4] = [0.0, 1.0, 1.0, 1.0];
/// `vec4_t colorWhite = {1, 1, 1, 1};`
#[no_mangle]
pub static mut colorWhite: [vec_t; 4] = [1.0, 1.0, 1.0, 1.0];
/// `vec4_t colorLtGrey = {0.75, 0.75, 0.75, 1};`
#[no_mangle]
pub static mut colorLtGrey: [vec_t; 4] = [0.75, 0.75, 0.75, 1.0];
/// `vec4_t colorMdGrey = {0.5, 0.5, 0.5, 1};`
#[no_mangle]
pub static mut colorMdGrey: [vec_t; 4] = [0.5, 0.5, 0.5, 1.0];
/// `vec4_t colorDkGrey = {0.25, 0.25, 0.25, 1};`
#[no_mangle]
pub static mut colorDkGrey: [vec_t; 4] = [0.25, 0.25, 0.25, 1.0];

/// ```c
/// vec4_t g_color_table[8] =
///     {
///     {0.0, 0.0, 0.0, 1.0},
///     {1.0, 0.0, 0.0, 1.0},
///     {0.0, 1.0, 0.0, 1.0},
///     {1.0, 1.0, 0.0, 1.0},
///     {0.0, 0.0, 1.0, 1.0},
///     {0.0, 1.0, 1.0, 1.0},
///     {1.0, 0.0, 1.0, 1.0},
///     {1.0, 1.0, 1.0, 1.0},
///     };
/// ```
#[no_mangle]
pub static mut g_color_table: [[vec_t; 4]; 8] = [
    [0.0, 0.0, 0.0, 1.0],
    [1.0, 0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0, 1.0],
    [1.0, 1.0, 0.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
    [0.0, 1.0, 1.0, 1.0],
    [1.0, 0.0, 1.0, 1.0],
    [1.0, 1.0, 1.0, 1.0],
];

/// `vec3_t bytedirs[NUMVERTEXNORMALS] = { ... };`
#[no_mangle]
pub static mut bytedirs: [[vec_t; 3]; NUMVERTEXNORMALS] = [
    [-0.525731f32, 0.000000f32, 0.850651f32],
    [-0.442863f32, 0.238856f32, 0.864188f32],
    [-0.295242f32, 0.000000f32, 0.955423f32],
    [-0.309017f32, 0.500000f32, 0.809017f32],
    [-0.162460f32, 0.262866f32, 0.951056f32],
    [0.000000f32, 0.000000f32, 1.000000f32],
    [0.000000f32, 0.850651f32, 0.525731f32],
    [-0.147621f32, 0.716567f32, 0.681718f32],
    [0.147621f32, 0.716567f32, 0.681718f32],
    [0.000000f32, 0.525731f32, 0.850651f32],
    [0.309017f32, 0.500000f32, 0.809017f32],
    [0.525731f32, 0.000000f32, 0.850651f32],
    [0.295242f32, 0.000000f32, 0.955423f32],
    [0.442863f32, 0.238856f32, 0.864188f32],
    [0.162460f32, 0.262866f32, 0.951056f32],
    [-0.681718f32, 0.147621f32, 0.716567f32],
    [-0.809017f32, 0.309017f32, 0.500000f32],
    [-0.587785f32, 0.425325f32, 0.688191f32],
    [-0.850651f32, 0.525731f32, 0.000000f32],
    [-0.864188f32, 0.442863f32, 0.238856f32],
    [-0.716567f32, 0.681718f32, 0.147621f32],
    [-0.688191f32, 0.587785f32, 0.425325f32],
    [-0.500000f32, 0.809017f32, 0.309017f32],
    [-0.238856f32, 0.864188f32, 0.442863f32],
    [-0.425325f32, 0.688191f32, 0.587785f32],
    [-0.716567f32, 0.681718f32, -0.147621f32],
    [-0.500000f32, 0.809017f32, -0.309017f32],
    [-0.525731f32, 0.850651f32, 0.000000f32],
    [0.000000f32, 0.850651f32, -0.525731f32],
    [-0.238856f32, 0.864188f32, -0.442863f32],
    [0.000000f32, 0.955423f32, -0.295242f32],
    [-0.262866f32, 0.951056f32, -0.162460f32],
    [0.000000f32, 1.000000f32, 0.000000f32],
    [0.000000f32, 0.955423f32, 0.295242f32],
    [-0.262866f32, 0.951056f32, 0.162460f32],
    [0.238856f32, 0.864188f32, 0.442863f32],
    [0.262866f32, 0.951056f32, 0.162460f32],
    [0.500000f32, 0.809017f32, 0.309017f32],
    [0.238856f32, 0.864188f32, -0.442863f32],
    [0.262866f32, 0.951056f32, -0.162460f32],
    [0.500000f32, 0.809017f32, -0.309017f32],
    [0.850651f32, 0.525731f32, 0.000000f32],
    [0.716567f32, 0.681718f32, 0.147621f32],
    [0.716567f32, 0.681718f32, -0.147621f32],
    [0.525731f32, 0.850651f32, 0.000000f32],
    [0.425325f32, 0.688191f32, 0.587785f32],
    [0.864188f32, 0.442863f32, 0.238856f32],
    [0.688191f32, 0.587785f32, 0.425325f32],
    [0.809017f32, 0.309017f32, 0.500000f32],
    [0.681718f32, 0.147621f32, 0.716567f32],
    [0.587785f32, 0.425325f32, 0.688191f32],
    [0.955423f32, 0.295242f32, 0.000000f32],
    [1.000000f32, 0.000000f32, 0.000000f32],
    [0.951056f32, 0.162460f32, 0.262866f32],
    [0.850651f32, -0.525731f32, 0.000000f32],
    [0.955423f32, -0.295242f32, 0.000000f32],
    [0.864188f32, -0.442863f32, 0.238856f32],
    [0.951056f32, -0.162460f32, 0.262866f32],
    [0.809017f32, -0.309017f32, 0.500000f32],
    [0.681718f32, -0.147621f32, 0.716567f32],
    [0.850651f32, 0.000000f32, 0.525731f32],
    [0.864188f32, 0.442863f32, -0.238856f32],
    [0.809017f32, 0.309017f32, -0.500000f32],
    [0.951056f32, 0.162460f32, -0.262866f32],
    [0.525731f32, 0.000000f32, -0.850651f32],
    [0.681718f32, 0.147621f32, -0.716567f32],
    [0.681718f32, -0.147621f32, -0.716567f32],
    [0.850651f32, 0.000000f32, -0.525731f32],
    [0.809017f32, -0.309017f32, -0.500000f32],
    [0.864188f32, -0.442863f32, -0.238856f32],
    [0.951056f32, -0.162460f32, -0.262866f32],
    [0.147621f32, 0.716567f32, -0.681718f32],
    [0.309017f32, 0.500000f32, -0.809017f32],
    [0.425325f32, 0.688191f32, -0.587785f32],
    [0.442863f32, 0.238856f32, -0.864188f32],
    [0.587785f32, 0.425325f32, -0.688191f32],
    [0.688191f32, 0.587785f32, -0.425325f32],
    [-0.147621f32, 0.716567f32, -0.681718f32],
    [-0.309017f32, 0.500000f32, -0.809017f32],
    [0.000000f32, 0.525731f32, -0.850651f32],
    [-0.525731f32, 0.000000f32, -0.850651f32],
    [-0.442863f32, 0.238856f32, -0.864188f32],
    [-0.295242f32, 0.000000f32, -0.955423f32],
    [-0.162460f32, 0.262866f32, -0.951056f32],
    [0.000000f32, 0.000000f32, -1.000000f32],
    [0.295242f32, 0.000000f32, -0.955423f32],
    [0.162460f32, 0.262866f32, -0.951056f32],
    [-0.442863f32, -0.238856f32, -0.864188f32],
    [-0.309017f32, -0.500000f32, -0.809017f32],
    [-0.162460f32, -0.262866f32, -0.951056f32],
    [0.000000f32, -0.850651f32, -0.525731f32],
    [-0.147621f32, -0.716567f32, -0.681718f32],
    [0.147621f32, -0.716567f32, -0.681718f32],
    [0.000000f32, -0.525731f32, -0.850651f32],
    [0.309017f32, -0.500000f32, -0.809017f32],
    [0.442863f32, -0.238856f32, -0.864188f32],
    [0.162460f32, -0.262866f32, -0.951056f32],
    [0.238856f32, -0.864188f32, -0.442863f32],
    [0.500000f32, -0.809017f32, -0.309017f32],
    [0.425325f32, -0.688191f32, -0.587785f32],
    [0.716567f32, -0.681718f32, -0.147621f32],
    [0.688191f32, -0.587785f32, -0.425325f32],
    [0.587785f32, -0.425325f32, -0.688191f32],
    [0.000000f32, -0.955423f32, -0.295242f32],
    [0.000000f32, -1.000000f32, 0.000000f32],
    [0.262866f32, -0.951056f32, -0.162460f32],
    [0.000000f32, -0.850651f32, 0.525731f32],
    [0.000000f32, -0.955423f32, 0.295242f32],
    [0.238856f32, -0.864188f32, 0.442863f32],
    [0.262866f32, -0.951056f32, 0.162460f32],
    [0.500000f32, -0.809017f32, 0.309017f32],
    [0.716567f32, -0.681718f32, 0.147621f32],
    [0.525731f32, -0.850651f32, 0.000000f32],
    [-0.238856f32, -0.864188f32, -0.442863f32],
    [-0.500000f32, -0.809017f32, -0.309017f32],
    [-0.262866f32, -0.951056f32, -0.162460f32],
    [-0.850651f32, -0.525731f32, 0.000000f32],
    [-0.716567f32, -0.681718f32, -0.147621f32],
    [-0.716567f32, -0.681718f32, 0.147621f32],
    [-0.525731f32, -0.850651f32, 0.000000f32],
    [-0.500000f32, -0.809017f32, 0.309017f32],
    [-0.238856f32, -0.864188f32, 0.442863f32],
    [-0.262866f32, -0.951056f32, 0.162460f32],
    [-0.864188f32, -0.442863f32, 0.238856f32],
    [-0.809017f32, -0.309017f32, 0.500000f32],
    [-0.688191f32, -0.587785f32, 0.425325f32],
    [-0.681718f32, -0.147621f32, 0.716567f32],
    [-0.442863f32, -0.238856f32, 0.864188f32],
    [-0.587785f32, -0.425325f32, 0.688191f32],
    [-0.309017f32, -0.500000f32, 0.809017f32],
    [-0.147621f32, -0.716567f32, 0.681718f32],
    [-0.425325f32, -0.688191f32, 0.587785f32],
    [-0.162460f32, -0.262866f32, 0.951056f32],
    [0.442863f32, -0.238856f32, 0.864188f32],
    [0.162460f32, -0.262866f32, 0.951056f32],
    [0.309017f32, -0.500000f32, 0.809017f32],
    [0.147621f32, -0.716567f32, 0.681718f32],
    [0.000000f32, -0.525731f32, 0.850651f32],
    [0.425325f32, -0.688191f32, 0.587785f32],
    [0.587785f32, -0.425325f32, 0.688191f32],
    [0.688191f32, -0.587785f32, 0.425325f32],
    [-0.955423f32, 0.295242f32, 0.000000f32],
    [-0.951056f32, 0.162460f32, 0.262866f32],
    [-1.000000f32, 0.000000f32, 0.000000f32],
    [-0.850651f32, 0.000000f32, 0.525731f32],
    [-0.955423f32, -0.295242f32, 0.000000f32],
    [-0.951056f32, -0.162460f32, 0.262866f32],
    [-0.864188f32, 0.442863f32, -0.238856f32],
    [-0.951056f32, 0.162460f32, -0.262866f32],
    [-0.809017f32, 0.309017f32, -0.500000f32],
    [-0.864188f32, -0.442863f32, -0.238856f32],
    [-0.951056f32, -0.162460f32, -0.262866f32],
    [-0.809017f32, -0.309017f32, -0.500000f32],
    [-0.681718f32, 0.147621f32, -0.716567f32],
    [-0.681718f32, -0.147621f32, -0.716567f32],
    [-0.850651f32, 0.000000f32, -0.525731f32],
    [-0.688191f32, 0.587785f32, -0.425325f32],
    [-0.587785f32, 0.425325f32, -0.688191f32],
    [-0.425325f32, 0.688191f32, -0.587785f32],
    [-0.425325f32, -0.688191f32, -0.587785f32],
    [-0.587785f32, -0.425325f32, -0.688191f32],
    [-0.688191f32, -0.587785f32, -0.425325f32],
];

//==============================================================

/// ```c
/// int Q_rand( int *seed ) {
///     *seed = (69069 * *seed + 1);
///     return *seed;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn Q_rand(seed: *mut c_int) -> c_int {
    // signed overflow wraps, exactly like the `imul`/`add` gcc emits
    *seed = 69069i32.wrapping_mul(*seed).wrapping_add(1);
    *seed
}

/// ```c
/// float Q_random( int *seed ) {
///     return ( Q_rand( seed ) & 0xffff ) / (float)0x10000;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn Q_random(seed: *mut c_int) -> f32 {
    (Q_rand(seed) & 0xffff) as f32 / 0x10000 as f32
}

/// ```c
/// float Q_crandom( int *seed ) {
///     return 2.0 * ( Q_random( seed ) - 0.5 );
/// }
/// ```
///
/// `2.0` and `0.5` are `double` literals, so the arithmetic happens in `f64`
/// and only the returned value is rounded back to `f32`.
#[no_mangle]
pub unsafe extern "C" fn Q_crandom(seed: *mut c_int) -> f32 {
    (2.0f64 * (Q_random(seed) as f64 - 0.5f64)) as f32
}

//=======================================================

/// ```c
/// signed char ClampChar( int i ) {
///     if ( i < -128 ) { return -128; }
///     if ( i > 127 ) { return 127; }
///     return i;
/// }
/// ```
#[no_mangle]
pub extern "C" fn ClampChar(i: c_int) -> i8 {
    if i < -128 {
        return -128;
    }
    if i > 127 {
        return 127;
    }
    i as i8
}

/// ```c
/// signed short ClampShort( int i ) {
///     if ( i < -32768 ) { return -32768; }
///     if ( i > 0x7fff ) { return 0x7fff; }
///     return i;
/// }
/// ```
#[no_mangle]
pub extern "C" fn ClampShort(i: c_int) -> i16 {
    if i < -32768 {
        return -32768;
    }
    if i > 0x7fff {
        return 0x7fff;
    }
    i as i16
}

/// ```c
/// // this isn't a real cheap function to call!
/// int DirToByte( vec3_t dir ) {
///     int     i, best;
///     float   d, bestd;
///
///     if ( !dir ) { return 0; }
///
///     bestd = 0;
///     best = 0;
///     for (i=0 ; i<NUMVERTEXNORMALS ; i++)
///     {
///         d = DotProduct (dir, bytedirs[i]);
///         if (d > bestd)
///         {
///             bestd = d;
///             best = i;
///         }
///     }
///
///     return best;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn DirToByte(dir: *mut vec_t) -> c_int {
    let mut best: c_int;
    let mut d: f32;
    let mut bestd: f32;

    if dir.is_null() {
        return 0;
    }

    bestd = 0.0;
    best = 0;
    let bytedirs_ptr = addr_of!(bytedirs) as *const vec_t;
    for i in 0..NUMVERTEXNORMALS as c_int {
        d = DotProduct(dir, bytedirs_ptr.add(3 * i as usize));
        if d > bestd {
            bestd = d;
            best = i;
        }
    }

    best
}

/// ```c
/// void ByteToDir( int b, vec3_t dir ) {
///     if ( b < 0 || b >= NUMVERTEXNORMALS ) {
///         VectorCopy( vec3_origin, dir );
///         return;
///     }
///     VectorCopy (bytedirs[b], dir);
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn ByteToDir(b: c_int, dir: *mut vec_t) {
    if b < 0 || b >= NUMVERTEXNORMALS as c_int {
        VectorCopy(addr_of!(vec3_origin) as *const vec_t, dir);
        return;
    }
    VectorCopy(
        (addr_of!(bytedirs) as *const vec_t).add(3 * b as usize),
        dir,
    );
}

/// ```c
/// unsigned ColorBytes3 (float r, float g, float b) {
///     unsigned    i;
///
///     ( (byte *)&i )[0] = r * 255;
///     ( (byte *)&i )[1] = g * 255;
///     ( (byte *)&i )[2] = b * 255;
///
///     return i;
/// }
/// ```
///
/// NOTE: `i` is never initialised and only three of its four bytes are written,
/// so the value of the most significant byte of the C result is indeterminate
/// (it is whatever the callee's stack slot happened to hold).  This translation
/// leaves it zero; the differential tests only compare the low 24 bits.
#[no_mangle]
pub extern "C" fn ColorBytes3(r: f32, g: f32, b: f32) -> c_uint {
    let mut i: [u8; 4] = [0; 4];

    i[0] = f32_to_byte(r * 255 as f32);
    i[1] = f32_to_byte(g * 255 as f32);
    i[2] = f32_to_byte(b * 255 as f32);

    u32::from_le_bytes(i)
}

/// ```c
/// unsigned ColorBytes4 (float r, float g, float b, float a) {
///     unsigned    i;
///
///     ( (byte *)&i )[0] = r * 255;
///     ( (byte *)&i )[1] = g * 255;
///     ( (byte *)&i )[2] = b * 255;
///     ( (byte *)&i )[3] = a * 255;
///
///     return i;
/// }
/// ```
#[no_mangle]
pub extern "C" fn ColorBytes4(r: f32, g: f32, b: f32, a: f32) -> c_uint {
    let mut i: [u8; 4] = [0; 4];

    i[0] = f32_to_byte(r * 255 as f32);
    i[1] = f32_to_byte(g * 255 as f32);
    i[2] = f32_to_byte(b * 255 as f32);
    i[3] = f32_to_byte(a * 255 as f32);

    u32::from_le_bytes(i)
}

/// ```c
/// float NormalizeColor( const vec3_t in, vec3_t out ) {
///     float   max;
///
///     max = in[0];
///     if ( in[1] > max ) { max = in[1]; }
///     if ( in[2] > max ) { max = in[2]; }
///
///     if ( !max ) {
///         VectorClear( out );
///     } else {
///         out[0] = in[0] / max;
///         out[1] = in[1] / max;
///         out[2] = in[2] / max;
///     }
///     return max;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn NormalizeColor(in_: *const vec_t, out: *mut vec_t) -> f32 {
    let mut max: f32;

    max = *in_.add(0);
    if *in_.add(1) > max {
        max = *in_.add(1);
    }
    if *in_.add(2) > max {
        max = *in_.add(2);
    }

    if max == 0.0 {
        VectorClear(out);
    } else {
        *out.add(0) = *in_.add(0) / max;
        *out.add(1) = *in_.add(1) / max;
        *out.add(2) = *in_.add(2) / max;
    }
    max
}

/// ```c
/// /*
/// =====================
/// PlaneFromPoints
///
/// Returns false if the triangle is degenrate.
/// The normal will point out of the clock for clockwise ordered points
/// =====================
/// */
/// qboolean PlaneFromPoints( vec4_t plane, const vec3_t a, const vec3_t b, const vec3_t c ) {
///     vec3_t  d1, d2;
///
///     VectorSubtract( b, a, d1 );
///     VectorSubtract( c, a, d2 );
///     CrossProduct( d2, d1, plane );
///     if ( VectorNormalize( plane ) == 0 ) {
///         return qfalse;
///     }
///
///     plane[3] = DotProduct( a, plane );
///     return qtrue;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn PlaneFromPoints(
    plane: *mut vec_t,
    a: *const vec_t,
    b: *const vec_t,
    c: *const vec_t,
) -> c_int {
    let mut d1: [vec_t; 3] = [0.0; 3];
    let mut d2: [vec_t; 3] = [0.0; 3];

    VectorSubtract(b, a, d1.as_mut_ptr());
    VectorSubtract(c, a, d2.as_mut_ptr());
    CrossProduct(d2.as_ptr(), d1.as_ptr(), plane);
    if VectorNormalize(plane) == 0.0 {
        return qfalse;
    }

    *plane.add(3) = DotProduct(a, plane);
    qtrue
}

/// ```c
/// /*
/// ===============
/// RotatePointAroundVector
///
/// This is not implemented very well...
/// ===============
/// */
/// void RotatePointAroundVector( vec3_t dst, const vec3_t dir, const vec3_t point,
///                              float degrees ) {
///     float   m[3][3];
///     float   im[3][3];
///     float   zrot[3][3];
///     float   tmpmat[3][3];
///     float   rot[3][3];
///     int i;
///     vec3_t vr, vup, vf;
///     float   rad;
///
///     vf[0] = dir[0];
///     vf[1] = dir[1];
///     vf[2] = dir[2];
///
///     PerpendicularVector( vr, dir );
///     CrossProduct( vr, vf, vup );
///
///     m[0][0] = vr[0];  m[1][0] = vr[1];  m[2][0] = vr[2];
///     m[0][1] = vup[0]; m[1][1] = vup[1]; m[2][1] = vup[2];
///     m[0][2] = vf[0];  m[1][2] = vf[1];  m[2][2] = vf[2];
///
///     memcpy( im, m, sizeof( im ) );
///
///     im[0][1] = m[1][0];
///     im[0][2] = m[2][0];
///     im[1][0] = m[0][1];
///     im[1][2] = m[2][1];
///     im[2][0] = m[0][2];
///     im[2][1] = m[1][2];
///
///     memset( zrot, 0, sizeof( zrot ) );
///     zrot[0][0] = zrot[1][1] = zrot[2][2] = 1.0F;
///
///     rad = DEG2RAD( degrees );
///     zrot[0][0] = cos( rad );
///     zrot[0][1] = sin( rad );
///     zrot[1][0] = -sin( rad );
///     zrot[1][1] = cos( rad );
///
///     MatrixMultiply( m, zrot, tmpmat );
///     MatrixMultiply( tmpmat, im, rot );
///
///     for ( i = 0; i < 3; i++ ) {
///         dst[i] = rot[i][0] * point[0] + rot[i][1] * point[1] + rot[i][2] * point[2];
///     }
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn RotatePointAroundVector(
    dst: *mut vec_t,
    dir: *const vec_t,
    point: *const vec_t,
    degrees: f32,
) {
    let mut m: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut im: [[f32; 3]; 3];
    let mut zrot: [[f32; 3]; 3];
    let mut tmpmat: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut rot: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut vr: [vec_t; 3] = [0.0; 3];
    let mut vup: [vec_t; 3] = [0.0; 3];
    let mut vf: [vec_t; 3] = [0.0; 3];
    let rad: f32;

    vf[0] = *dir.add(0);
    vf[1] = *dir.add(1);
    vf[2] = *dir.add(2);

    PerpendicularVector(vr.as_mut_ptr(), dir);
    CrossProduct(vr.as_ptr(), vf.as_ptr(), vup.as_mut_ptr());

    m[0][0] = vr[0];
    m[1][0] = vr[1];
    m[2][0] = vr[2];

    m[0][1] = vup[0];
    m[1][1] = vup[1];
    m[2][1] = vup[2];

    m[0][2] = vf[0];
    m[1][2] = vf[1];
    m[2][2] = vf[2];

    im = m; // memcpy( im, m, sizeof( im ) );

    im[0][1] = m[1][0];
    im[0][2] = m[2][0];
    im[1][0] = m[0][1];
    im[1][2] = m[2][1];
    im[2][0] = m[0][2];
    im[2][1] = m[1][2];

    zrot = [[0.0; 3]; 3]; // memset( zrot, 0, sizeof( zrot ) );
    zrot[2][2] = 1.0f32;
    zrot[1][1] = zrot[2][2];
    zrot[0][0] = zrot[1][1];

    rad = DEG2RAD(degrees) as f32;
    zrot[0][0] = (rad as f64).cos() as f32;
    zrot[0][1] = (rad as f64).sin() as f32;
    zrot[1][0] = (-(rad as f64).sin()) as f32;
    zrot[1][1] = (rad as f64).cos() as f32;

    MatrixMultiply(m.as_mut_ptr(), zrot.as_mut_ptr(), tmpmat.as_mut_ptr());
    MatrixMultiply(tmpmat.as_mut_ptr(), im.as_mut_ptr(), rot.as_mut_ptr());

    for i in 0..3usize {
        *dst.add(i) = rot[i][0] * *point.add(0) + rot[i][1] * *point.add(1) + rot[i][2] * *point.add(2);
    }
}

/// ```c
/// /*
/// ===============
/// RotateAroundDirection
/// ===============
/// */
/// void RotateAroundDirection( vec3_t axis[3], float yaw ) {
///
///     // create an arbitrary axis[1]
///     PerpendicularVector( axis[1], axis[0] );
///
///     // rotate it around axis[0] by yaw
///     if ( yaw ) {
///         vec3_t  temp;
///
///         VectorCopy( axis[1], temp );
///         RotatePointAroundVector( axis[1], axis[0], temp, yaw );
///     }
///
///     // cross to get axis[2]
///     CrossProduct( axis[0], axis[1], axis[2] );
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn RotateAroundDirection(axis: *mut [vec_t; 3], yaw: f32) {
    // create an arbitrary axis[1]
    PerpendicularVector(
        (*axis.add(1)).as_mut_ptr(),
        (*axis.add(0)).as_ptr(),
    );

    // rotate it around axis[0] by yaw
    if yaw != 0.0 {
        let mut temp: [vec_t; 3] = [0.0; 3];

        VectorCopy((*axis.add(1)).as_ptr(), temp.as_mut_ptr());
        RotatePointAroundVector(
            (*axis.add(1)).as_mut_ptr(),
            (*axis.add(0)).as_ptr(),
            temp.as_ptr(),
            yaw,
        );
    }

    // cross to get axis[2]
    CrossProduct(
        (*axis.add(0)).as_ptr(),
        (*axis.add(1)).as_ptr(),
        (*axis.add(2)).as_mut_ptr(),
    );
}

/// ```c
/// void vectoangles( const vec3_t value1, vec3_t angles ) {
///     float   forward;
///     float   yaw, pitch;
///
///     if ( value1[1] == 0 && value1[0] == 0 ) {
///         yaw = 0;
///         if ( value1[2] > 0 ) { pitch = 90; }
///         else { pitch = 270; }
///     }
///     else {
///         if ( value1[0] ) {
///             yaw = ( atan2 ( value1[1], value1[0] ) * 180 / M_PI );
///         }
///         else if ( value1[1] > 0 ) { yaw = 90; }
///         else { yaw = 270; }
///         if ( yaw < 0 ) { yaw += 360; }
///
///         forward = sqrt ( value1[0]*value1[0] + value1[1]*value1[1] );
///         pitch = ( atan2(value1[2], forward) * 180 / M_PI );
///         if ( pitch < 0 ) { pitch += 360; }
///     }
///
///     angles[PITCH] = -pitch;
///     angles[YAW] = yaw;
///     angles[ROLL] = 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vectoangles(value1: *const vec_t, angles: *mut vec_t) {
    let forward: f32;
    let mut yaw: f32;
    let mut pitch: f32;

    if *value1.add(1) == 0.0 && *value1.add(0) == 0.0 {
        yaw = 0.0;
        if *value1.add(2) > 0.0 {
            pitch = 90.0;
        } else {
            pitch = 270.0;
        }
    } else {
        if *value1.add(0) != 0.0 {
            yaw = ((*value1.add(1) as f64).atan2(*value1.add(0) as f64) * 180.0 / M_PI) as f32;
        } else if *value1.add(1) > 0.0 {
            yaw = 90.0;
        } else {
            yaw = 270.0;
        }
        if yaw < 0.0 {
            yaw += 360.0;
        }

        forward = ((*value1.add(0) * *value1.add(0) + *value1.add(1) * *value1.add(1)) as f64)
            .sqrt() as f32;
        pitch = ((*value1.add(2) as f64).atan2(forward as f64) * 180.0 / M_PI) as f32;
        if pitch < 0.0 {
            pitch += 360.0;
        }
    }

    *angles.add(PITCH) = -pitch;
    *angles.add(YAW) = yaw;
    *angles.add(ROLL) = 0.0;
}

/// ```c
/// /*
/// =================
/// AnglesToAxis
/// =================
/// */
/// void AnglesToAxis( const vec3_t angles, vec3_t axis[3] ) {
///     vec3_t  right;
///
///     // angle vectors returns "right" instead of "y axis"
///     AngleVectors( angles, axis[0], right, axis[2] );
///     VectorSubtract( vec3_origin, right, axis[1] );
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn AnglesToAxis(angles: *const vec_t, axis: *mut [vec_t; 3]) {
    let mut right: [vec_t; 3] = [0.0; 3];

    // angle vectors returns "right" instead of "y axis"
    AngleVectors(
        angles,
        (*axis.add(0)).as_mut_ptr(),
        right.as_mut_ptr(),
        (*axis.add(2)).as_mut_ptr(),
    );
    VectorSubtract(
        addr_of!(vec3_origin) as *const vec_t,
        right.as_ptr(),
        (*axis.add(1)).as_mut_ptr(),
    );
}

/// ```c
/// void AxisClear( vec3_t axis[3] ) {
///     axis[0][0] = 1; axis[0][1] = 0; axis[0][2] = 0;
///     axis[1][0] = 0; axis[1][1] = 1; axis[1][2] = 0;
///     axis[2][0] = 0; axis[2][1] = 0; axis[2][2] = 1;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn AxisClear(axis: *mut [vec_t; 3]) {
    (*axis.add(0))[0] = 1.0;
    (*axis.add(0))[1] = 0.0;
    (*axis.add(0))[2] = 0.0;
    (*axis.add(1))[0] = 0.0;
    (*axis.add(1))[1] = 1.0;
    (*axis.add(1))[2] = 0.0;
    (*axis.add(2))[0] = 0.0;
    (*axis.add(2))[1] = 0.0;
    (*axis.add(2))[2] = 1.0;
}

/// ```c
/// void AxisCopy( vec3_t in[3], vec3_t out[3] ) {
///     VectorCopy( in[0], out[0] );
///     VectorCopy( in[1], out[1] );
///     VectorCopy( in[2], out[2] );
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn AxisCopy(in_: *mut [vec_t; 3], out: *mut [vec_t; 3]) {
    VectorCopy((*in_.add(0)).as_ptr(), (*out.add(0)).as_mut_ptr());
    VectorCopy((*in_.add(1)).as_ptr(), (*out.add(1)).as_mut_ptr());
    VectorCopy((*in_.add(2)).as_ptr(), (*out.add(2)).as_mut_ptr());
}

/// ```c
/// void ProjectPointOnPlane( vec3_t dst, const vec3_t p, const vec3_t normal )
/// {
///     float d;
///     vec3_t n;
///     float inv_denom;
///
///     inv_denom =  DotProduct( normal, normal );
///     inv_denom = 1.0f / inv_denom;
///
///     d = DotProduct( normal, p ) * inv_denom;
///
///     n[0] = normal[0] * inv_denom;
///     n[1] = normal[1] * inv_denom;
///     n[2] = normal[2] * inv_denom;
///
///     dst[0] = p[0] - d * n[0];
///     dst[1] = p[1] - d * n[1];
///     dst[2] = p[2] - d * n[2];
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn ProjectPointOnPlane(
    dst: *mut vec_t,
    p: *const vec_t,
    normal: *const vec_t,
) {
    let d: f32;
    let mut n: [vec_t; 3] = [0.0; 3];
    let mut inv_denom: f32;

    inv_denom = DotProduct(normal, normal);
    inv_denom = 1.0f32 / inv_denom;

    d = DotProduct(normal, p) * inv_denom;

    n[0] = *normal.add(0) * inv_denom;
    n[1] = *normal.add(1) * inv_denom;
    n[2] = *normal.add(2) * inv_denom;

    *dst.add(0) = *p.add(0) - d * n[0];
    *dst.add(1) = *p.add(1) - d * n[1];
    *dst.add(2) = *p.add(2) - d * n[2];
}

/// ```c
/// /*
/// ================
/// MakeNormalVectors
///
/// Given a normalized forward vector, create two
/// other perpendicular vectors
/// ================
/// */
/// void MakeNormalVectors( const vec3_t forward, vec3_t right, vec3_t up) {
///     float       d;
///
///     // this rotate and negate guarantees a vector
///     // not colinear with the original
///     right[1] = -forward[0];
///     right[2] = forward[1];
///     right[0] = forward[2];
///
///     d = DotProduct (right, forward);
///     VectorMA (right, -d, forward, right);
///     VectorNormalize (right);
///     CrossProduct (right, forward, up);
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn MakeNormalVectors(
    forward: *const vec_t,
    right: *mut vec_t,
    up: *mut vec_t,
) {
    let d: f32;

    // this rotate and negate guarantees a vector
    // not colinear with the original
    *right.add(1) = -*forward.add(0);
    *right.add(2) = *forward.add(1);
    *right.add(0) = *forward.add(2);

    d = DotProduct(right, forward);
    VectorMA(right, -d, forward, right);
    VectorNormalize(right);
    CrossProduct(right, forward, up);
}

/// ```c
/// void VectorRotate( vec3_t in, vec3_t matrix[3], vec3_t out )
/// {
///     out[0] = DotProduct( in, matrix[0] );
///     out[1] = DotProduct( in, matrix[1] );
///     out[2] = DotProduct( in, matrix[2] );
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn VectorRotate(
    in_: *mut vec_t,
    matrix: *mut [vec_t; 3],
    out: *mut vec_t,
) {
    *out.add(0) = DotProduct(in_, (*matrix.add(0)).as_ptr());
    *out.add(1) = DotProduct(in_, (*matrix.add(1)).as_ptr());
    *out.add(2) = DotProduct(in_, (*matrix.add(2)).as_ptr());
}

//============================================================================

// #if !idppc  --  `#define idppc 0`, so this block is compiled.

/// ```c
/// /*
/// ** float q_rsqrt( float number )
/// */
/// float Q_rsqrt( float number )
/// {
///     uint32_t i;
///     float x2, y;
///     const float threehalfs = 1.5F;
///
///     x2 = number * 0.5F;
///     y  = number;
///
///     memcpy(&i, &y, sizeof(float));      // evil floating point bit level hacking
///     i  = 0x5f3759dfu - (i >> 1);        // what the?
///     memcpy(&y, &i, sizeof(float));
///
///     y  = y * (threehalfs - (x2 * y * y));   // 1st iteration
///
///     return y;
/// }
/// ```
#[no_mangle]
pub extern "C" fn Q_rsqrt(number: f32) -> f32 {
    let mut i: u32;
    let x2: f32;
    let mut y: f32;
    let threehalfs: f32 = 1.5f32;

    x2 = number * 0.5f32;
    y = number;

    i = y.to_bits(); // evil floating point bit level hacking
    i = 0x5f3759dfu32.wrapping_sub(i >> 1); // what the?
    y = f32::from_bits(i);

    y = y * (threehalfs - (x2 * y * y)); // 1st iteration

    y
}

/// ```c
/// float Q_fabs( float f ) {
///     int tmp = * ( int * ) &f;
///     tmp &= 0x7FFFFFFF;
///     return * ( float * ) &tmp;
/// }
/// ```
#[no_mangle]
pub extern "C" fn Q_fabs(f: f32) -> f32 {
    let mut tmp: i32 = f.to_bits() as i32;
    tmp &= 0x7FFFFFFF;
    f32::from_bits(tmp as u32)
}

//============================================================

/// ```c
/// /*
/// ===============
/// LerpAngle
/// ===============
/// */
/// float LerpAngle (float from, float to, float frac) {
///     float   a;
///
///     if ( to - from > 180 ) { to -= 360; }
///     if ( to - from < -180 ) { to += 360; }
///     a = from + frac * (to - from);
///
///     return a;
/// }
/// ```
#[no_mangle]
pub extern "C" fn LerpAngle(from: f32, mut to: f32, frac: f32) -> f32 {
    let a: f32;

    if to - from > 180.0 {
        to -= 360.0;
    }
    if to - from < -180.0 {
        to += 360.0;
    }
    a = from + frac * (to - from);

    a
}

/// ```c
/// /*
/// =================
/// AngleSubtract
///
/// Always returns a value from -180 to 180
/// =================
/// */
/// float   AngleSubtract( float a1, float a2 ) {
///     float   a;
///
///     a = a1 - a2;
///     while ( a > 180 ) { a -= 360; }
///     while ( a < -180 ) { a += 360; }
///     return a;
/// }
/// ```
///
/// NOTE: just like the C original this loops forever when `a1 - a2` is infinite
/// or so large that subtracting 360 does not change it (|a| >= 2^28 or so).
#[no_mangle]
pub extern "C" fn AngleSubtract(a1: f32, a2: f32) -> f32 {
    let mut a: f32;

    a = a1 - a2;
    while a > 180.0 {
        a -= 360.0;
    }
    while a < -180.0 {
        a += 360.0;
    }
    a
}

/// ```c
/// void AnglesSubtract( vec3_t v1, vec3_t v2, vec3_t v3 ) {
///     v3[0] = AngleSubtract( v1[0], v2[0] );
///     v3[1] = AngleSubtract( v1[1], v2[1] );
///     v3[2] = AngleSubtract( v1[2], v2[2] );
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn AnglesSubtract(v1: *mut vec_t, v2: *mut vec_t, v3: *mut vec_t) {
    *v3.add(0) = AngleSubtract(*v1.add(0), *v2.add(0));
    *v3.add(1) = AngleSubtract(*v1.add(1), *v2.add(1));
    *v3.add(2) = AngleSubtract(*v1.add(2), *v2.add(2));
}

/// ```c
/// float   AngleMod(float a) {
///     a = (360.0/65536) * ((int)(a*(65536/360.0)) & 65535);
///     return a;
/// }
/// ```
///
/// `65536/360.0` and `360.0/65536` are `double` constants, so the products are
/// evaluated in `f64` and the `(int)` conversion is a `cvttsd2si`.
#[no_mangle]
pub extern "C" fn AngleMod(a: f32) -> f32 {
    ((360.0f64 / 65536.0f64) * (f64_to_i32(a as f64 * (65536.0f64 / 360.0f64)) & 65535) as f64)
        as f32
}

/// ```c
/// /*
/// =================
/// AngleNormalize360
///
/// returns angle normalized to the range [0 <= angle < 360]
/// =================
/// */
/// float AngleNormalize360 ( float angle ) {
///     return (360.0 / 65536) * ((int)(angle * (65536 / 360.0)) & 65535);
/// }
/// ```
#[no_mangle]
pub extern "C" fn AngleNormalize360(angle: f32) -> f32 {
    ((360.0f64 / 65536.0f64) * (f64_to_i32(angle as f64 * (65536.0f64 / 360.0f64)) & 65535) as f64)
        as f32
}

/// ```c
/// /*
/// =================
/// AngleNormalize180
///
/// returns angle normalized to the range [-180 < angle <= 180]
/// =================
/// */
/// float AngleNormalize180 ( float angle ) {
///     angle = AngleNormalize360( angle );
///     if ( angle > 180.0 ) {
///         angle -= 360.0;
///     }
///     return angle;
/// }
/// ```
#[no_mangle]
pub extern "C" fn AngleNormalize180(angle: f32) -> f32 {
    let mut angle = AngleNormalize360(angle);
    if angle as f64 > 180.0f64 {
        // `360.0` is a double literal: the subtraction happens in f64 and the
        // result is rounded back to f32 on assignment.
        angle = (angle as f64 - 360.0f64) as f32;
    }
    angle
}

/// ```c
/// /*
/// =================
/// AngleDelta
///
/// returns the normalized delta from angle1 to angle2
/// =================
/// */
/// float AngleDelta ( float angle1, float angle2 ) {
///     return AngleNormalize180( angle1 - angle2 );
/// }
/// ```
#[no_mangle]
pub extern "C" fn AngleDelta(angle1: f32, angle2: f32) -> f32 {
    AngleNormalize180(angle1 - angle2)
}

//============================================================

/// ```c
/// /*
/// =================
/// SetPlaneSignbits
/// =================
/// */
/// void SetPlaneSignbits (cplane_t *out) {
///     int bits, j;
///
///     // for fast box on planeside test
///     bits = 0;
///     for (j=0 ; j<3 ; j++) {
///         if (out->normal[j] < 0) {
///             bits |= 1<<j;
///         }
///     }
///     out->signbits = bits;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn SetPlaneSignbits(out: *mut cplane_t) {
    let mut bits: c_int;

    // for fast box on planeside test
    bits = 0;
    for j in 0..3usize {
        if (*out).normal[j] < 0.0 {
            bits |= 1 << j;
        }
    }
    (*out).signbits = bits as u8;
}

/// ```c
/// /*
/// ==================
/// BoxOnPlaneSide
///
/// Returns 1, 2, or 1 + 2
/// ==================
/// */
/// int BoxOnPlaneSide (vec3_t emins, vec3_t emaxs, struct cplane_s *p)
/// {
///     float   dist1, dist2;
///     int     sides;
///
/// // fast axial cases
///     if (p->type < 3)
///     {
///         if (p->dist <= emins[p->type]) return 1;
///         if (p->dist >= emaxs[p->type]) return 2;
///         return 3;
///     }
///
/// // general case
///     switch (p->signbits)
///     {
///     case 0: ... case 7: ...
///     default:
///         dist1 = dist2 = 0;      // shut up compiler
///         break;
///     }
///
///     sides = 0;
///     if (dist1 >= p->dist) sides = 1;
///     if (dist2 < p->dist) sides |= 2;
///
///     return sides;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn BoxOnPlaneSide(
    emins: *mut vec_t,
    emaxs: *mut vec_t,
    p: *mut cplane_t,
) -> c_int {
    let dist1: f32;
    let dist2: f32;
    let mut sides: c_int;

    let normal = (*p).normal;
    let mins = |i: usize| *emins.add(i);
    let maxs = |i: usize| *emaxs.add(i);

    // fast axial cases
    if ((*p).type_ as c_int) < 3 {
        if (*p).dist <= mins((*p).type_ as usize) {
            return 1;
        }
        if (*p).dist >= maxs((*p).type_ as usize) {
            return 2;
        }
        return 3;
    }

    // general case
    match (*p).signbits {
        0 => {
            dist1 = normal[0] * maxs(0) + normal[1] * maxs(1) + normal[2] * maxs(2);
            dist2 = normal[0] * mins(0) + normal[1] * mins(1) + normal[2] * mins(2);
        }
        1 => {
            dist1 = normal[0] * mins(0) + normal[1] * maxs(1) + normal[2] * maxs(2);
            dist2 = normal[0] * maxs(0) + normal[1] * mins(1) + normal[2] * mins(2);
        }
        2 => {
            dist1 = normal[0] * maxs(0) + normal[1] * mins(1) + normal[2] * maxs(2);
            dist2 = normal[0] * mins(0) + normal[1] * maxs(1) + normal[2] * mins(2);
        }
        3 => {
            dist1 = normal[0] * mins(0) + normal[1] * mins(1) + normal[2] * maxs(2);
            dist2 = normal[0] * maxs(0) + normal[1] * maxs(1) + normal[2] * mins(2);
        }
        4 => {
            dist1 = normal[0] * maxs(0) + normal[1] * maxs(1) + normal[2] * mins(2);
            dist2 = normal[0] * mins(0) + normal[1] * mins(1) + normal[2] * maxs(2);
        }
        5 => {
            dist1 = normal[0] * mins(0) + normal[1] * maxs(1) + normal[2] * mins(2);
            dist2 = normal[0] * maxs(0) + normal[1] * mins(1) + normal[2] * maxs(2);
        }
        6 => {
            dist1 = normal[0] * maxs(0) + normal[1] * mins(1) + normal[2] * mins(2);
            dist2 = normal[0] * mins(0) + normal[1] * maxs(1) + normal[2] * maxs(2);
        }
        7 => {
            dist1 = normal[0] * mins(0) + normal[1] * mins(1) + normal[2] * mins(2);
            dist2 = normal[0] * maxs(0) + normal[1] * maxs(1) + normal[2] * maxs(2);
        }
        _ => {
            dist2 = 0.0; // shut up compiler
            dist1 = dist2;
        }
    }

    sides = 0;
    if dist1 >= (*p).dist {
        sides = 1;
    }
    if dist2 < (*p).dist {
        sides |= 2;
    }

    sides
}

/// ```c
/// /*
/// =================
/// RadiusFromBounds
/// =================
/// */
/// float RadiusFromBounds( const vec3_t mins, const vec3_t maxs ) {
///     int     i;
///     vec3_t  corner;
///     float   a, b;
///
///     for (i=0 ; i<3 ; i++) {
///         a = fabs( mins[i] );
///         b = fabs( maxs[i] );
///         corner[i] = a > b ? a : b;
///     }
///
///     return VectorLength (corner);
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn RadiusFromBounds(mins: *const vec_t, maxs: *const vec_t) -> f32 {
    let mut corner: [vec_t; 3] = [0.0; 3];
    let mut a: f32;
    let mut b: f32;

    for i in 0..3usize {
        a = (*mins.add(i) as f64).abs() as f32;
        b = (*maxs.add(i) as f64).abs() as f32;
        corner[i] = if a > b { a } else { b };
    }

    VectorLength(corner.as_ptr())
}

/// ```c
/// void ClearBounds( vec3_t mins, vec3_t maxs ) {
///     mins[0] = mins[1] = mins[2] = 99999;
///     maxs[0] = maxs[1] = maxs[2] = -99999;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn ClearBounds(mins: *mut vec_t, maxs: *mut vec_t) {
    *mins.add(2) = 99999.0;
    *mins.add(1) = *mins.add(2);
    *mins.add(0) = *mins.add(1);
    *maxs.add(2) = -99999.0;
    *maxs.add(1) = *maxs.add(2);
    *maxs.add(0) = *maxs.add(1);
}

/// ```c
/// void AddPointToBounds( const vec3_t v, vec3_t mins, vec3_t maxs ) {
///     if ( v[0] < mins[0] ) { mins[0] = v[0]; }
///     if ( v[0] > maxs[0]) { maxs[0] = v[0]; }
///
///     if ( v[1] < mins[1] ) { mins[1] = v[1]; }
///     if ( v[1] > maxs[1]) { maxs[1] = v[1]; }
///
///     if ( v[2] < mins[2] ) { mins[2] = v[2]; }
///     if ( v[2] > maxs[2]) { maxs[2] = v[2]; }
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn AddPointToBounds(v: *const vec_t, mins: *mut vec_t, maxs: *mut vec_t) {
    if *v.add(0) < *mins.add(0) {
        *mins.add(0) = *v.add(0);
    }
    if *v.add(0) > *maxs.add(0) {
        *maxs.add(0) = *v.add(0);
    }

    if *v.add(1) < *mins.add(1) {
        *mins.add(1) = *v.add(1);
    }
    if *v.add(1) > *maxs.add(1) {
        *maxs.add(1) = *v.add(1);
    }

    if *v.add(2) < *mins.add(2) {
        *mins.add(2) = *v.add(2);
    }
    if *v.add(2) > *maxs.add(2) {
        *maxs.add(2) = *v.add(2);
    }
}

/// ```c
/// vec_t VectorNormalize( vec3_t v ) {
///     // NOTE: TTimo - Apple G4 altivec source uses double?
///     float   length, ilength;
///
///     length = v[0]*v[0] + v[1]*v[1] + v[2]*v[2];
///     length = sqrt (length);
///
///     if ( length ) {
///         ilength = 1/length;
///         v[0] *= ilength;
///         v[1] *= ilength;
///         v[2] *= ilength;
///     }
///
///     return length;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn VectorNormalize(v: *mut vec_t) -> vec_t {
    let mut length: f32;
    let ilength: f32;

    length = *v.add(0) * *v.add(0) + *v.add(1) * *v.add(1) + *v.add(2) * *v.add(2);
    length = (length as f64).sqrt() as f32;

    if length != 0.0 {
        ilength = 1.0 / length;
        *v.add(0) = *v.add(0) * ilength;
        *v.add(1) = *v.add(1) * ilength;
        *v.add(2) = *v.add(2) * ilength;
    }

    length
}

/// ```c
/// vec_t VectorNormalize2( const vec3_t v, vec3_t out) {
///     float   length, ilength;
///
///     length = v[0]*v[0] + v[1]*v[1] + v[2]*v[2];
///     length = sqrt (length);
///
///     if (length)
///     {
///         ilength = 1/length;
///         out[0] = v[0]*ilength;
///         out[1] = v[1]*ilength;
///         out[2] = v[2]*ilength;
///     } else {
///         VectorClear( out );
///     }
///
///     return length;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn VectorNormalize2(v: *const vec_t, out: *mut vec_t) -> vec_t {
    let mut length: f32;
    let ilength: f32;

    length = *v.add(0) * *v.add(0) + *v.add(1) * *v.add(1) + *v.add(2) * *v.add(2);
    length = (length as f64).sqrt() as f32;

    if length != 0.0 {
        ilength = 1.0 / length;
        *out.add(0) = *v.add(0) * ilength;
        *out.add(1) = *v.add(1) * ilength;
        *out.add(2) = *v.add(2) * ilength;
    } else {
        VectorClear(out);
    }

    length
}

/// ```c
/// void _VectorMA( const vec3_t veca, float scale, const vec3_t vecb, vec3_t vecc) {
///     vecc[0] = veca[0] + scale*vecb[0];
///     vecc[1] = veca[1] + scale*vecb[1];
///     vecc[2] = veca[2] + scale*vecb[2];
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn _VectorMA(
    veca: *const vec_t,
    scale: f32,
    vecb: *const vec_t,
    vecc: *mut vec_t,
) {
    *vecc.add(0) = *veca.add(0) + scale * *vecb.add(0);
    *vecc.add(1) = *veca.add(1) + scale * *vecb.add(1);
    *vecc.add(2) = *veca.add(2) + scale * *vecb.add(2);
}

/// ```c
/// vec_t _DotProduct( const vec3_t v1, const vec3_t v2 ) {
///     return v1[0]*v2[0] + v1[1]*v2[1] + v1[2]*v2[2];
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn _DotProduct(v1: *const vec_t, v2: *const vec_t) -> vec_t {
    *v1.add(0) * *v2.add(0) + *v1.add(1) * *v2.add(1) + *v1.add(2) * *v2.add(2)
}

/// ```c
/// void _VectorSubtract( const vec3_t veca, const vec3_t vecb, vec3_t out ) {
///     out[0] = veca[0]-vecb[0];
///     out[1] = veca[1]-vecb[1];
///     out[2] = veca[2]-vecb[2];
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn _VectorSubtract(veca: *const vec_t, vecb: *const vec_t, out: *mut vec_t) {
    *out.add(0) = *veca.add(0) - *vecb.add(0);
    *out.add(1) = *veca.add(1) - *vecb.add(1);
    *out.add(2) = *veca.add(2) - *vecb.add(2);
}

/// ```c
/// void _VectorAdd( const vec3_t veca, const vec3_t vecb, vec3_t out ) {
///     out[0] = veca[0]+vecb[0];
///     out[1] = veca[1]+vecb[1];
///     out[2] = veca[2]+vecb[2];
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn _VectorAdd(veca: *const vec_t, vecb: *const vec_t, out: *mut vec_t) {
    *out.add(0) = *veca.add(0) + *vecb.add(0);
    *out.add(1) = *veca.add(1) + *vecb.add(1);
    *out.add(2) = *veca.add(2) + *vecb.add(2);
}

/// ```c
/// void _VectorCopy( const vec3_t in, vec3_t out ) {
///     out[0] = in[0];
///     out[1] = in[1];
///     out[2] = in[2];
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn _VectorCopy(in_: *const vec_t, out: *mut vec_t) {
    *out.add(0) = *in_.add(0);
    *out.add(1) = *in_.add(1);
    *out.add(2) = *in_.add(2);
}

/// ```c
/// void _VectorScale( const vec3_t in, vec_t scale, vec3_t out ) {
///     out[0] = in[0]*scale;
///     out[1] = in[1]*scale;
///     out[2] = in[2]*scale;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn _VectorScale(in_: *const vec_t, scale: vec_t, out: *mut vec_t) {
    *out.add(0) = *in_.add(0) * scale;
    *out.add(1) = *in_.add(1) * scale;
    *out.add(2) = *in_.add(2) * scale;
}

/// ```c
/// void Vector4Scale( const vec4_t in, vec_t scale, vec4_t out ) {
///     out[0] = in[0]*scale;
///     out[1] = in[1]*scale;
///     out[2] = in[2]*scale;
///     out[3] = in[3]*scale;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn Vector4Scale(in_: *const vec_t, scale: vec_t, out: *mut vec_t) {
    *out.add(0) = *in_.add(0) * scale;
    *out.add(1) = *in_.add(1) * scale;
    *out.add(2) = *in_.add(2) * scale;
    *out.add(3) = *in_.add(3) * scale;
}

/// ```c
/// int Q_log2( int val ) {
///     int answer;
///
///     answer = 0;
///     while ( ( val>>=1 ) != 0 ) {
///         answer++;
///     }
///     return answer;
/// }
/// ```
///
/// NOTE: `>>` on a negative `int` is an arithmetic shift, so -- exactly like the
/// C original -- this never terminates for a negative argument (the value gets
/// stuck at -1).
#[no_mangle]
pub extern "C" fn Q_log2(mut val: c_int) -> c_int {
    let mut answer: c_int;

    answer = 0;
    loop {
        val >>= 1;
        if val == 0 {
            break;
        }
        answer += 1;
    }
    answer
}

/// ```c
/// /*
/// ================
/// MatrixMultiply
/// ================
/// */
/// void MatrixMultiply(float in1[3][3], float in2[3][3], float out[3][3]) {
///     out[0][0] = in1[0][0] * in2[0][0] + in1[0][1] * in2[1][0] + in1[0][2] * in2[2][0];
///     out[0][1] = in1[0][0] * in2[0][1] + in1[0][1] * in2[1][1] + in1[0][2] * in2[2][1];
///     out[0][2] = in1[0][0] * in2[0][2] + in1[0][1] * in2[1][2] + in1[0][2] * in2[2][2];
///     out[1][0] = in1[1][0] * in2[0][0] + in1[1][1] * in2[1][0] + in1[1][2] * in2[2][0];
///     out[1][1] = in1[1][0] * in2[0][1] + in1[1][1] * in2[1][1] + in1[1][2] * in2[2][1];
///     out[1][2] = in1[1][0] * in2[0][2] + in1[1][1] * in2[1][2] + in1[1][2] * in2[2][2];
///     out[2][0] = in1[2][0] * in2[0][0] + in1[2][1] * in2[1][0] + in1[2][2] * in2[2][0];
///     out[2][1] = in1[2][0] * in2[0][1] + in1[2][1] * in2[1][1] + in1[2][2] * in2[2][1];
///     out[2][2] = in1[2][0] * in2[0][2] + in1[2][1] * in2[1][2] + in1[2][2] * in2[2][2];
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn MatrixMultiply(
    in1: *mut [f32; 3],
    in2: *mut [f32; 3],
    out: *mut [f32; 3],
) {
    let a = |i: usize, j: usize| (*in1.add(i))[j];
    let b = |i: usize, j: usize| (*in2.add(i))[j];

    (*out.add(0))[0] = a(0, 0) * b(0, 0) + a(0, 1) * b(1, 0) + a(0, 2) * b(2, 0);
    (*out.add(0))[1] = a(0, 0) * b(0, 1) + a(0, 1) * b(1, 1) + a(0, 2) * b(2, 1);
    (*out.add(0))[2] = a(0, 0) * b(0, 2) + a(0, 1) * b(1, 2) + a(0, 2) * b(2, 2);
    (*out.add(1))[0] = a(1, 0) * b(0, 0) + a(1, 1) * b(1, 0) + a(1, 2) * b(2, 0);
    (*out.add(1))[1] = a(1, 0) * b(0, 1) + a(1, 1) * b(1, 1) + a(1, 2) * b(2, 1);
    (*out.add(1))[2] = a(1, 0) * b(0, 2) + a(1, 1) * b(1, 2) + a(1, 2) * b(2, 2);
    (*out.add(2))[0] = a(2, 0) * b(0, 0) + a(2, 1) * b(1, 0) + a(2, 2) * b(2, 0);
    (*out.add(2))[1] = a(2, 0) * b(0, 1) + a(2, 1) * b(1, 1) + a(2, 2) * b(2, 1);
    (*out.add(2))[2] = a(2, 0) * b(0, 2) + a(2, 1) * b(1, 2) + a(2, 2) * b(2, 2);
}

/// ```c
/// void AngleVectors( const vec3_t angles, vec3_t forward, vec3_t right, vec3_t up) {
///     float       angle;
///     static float        sr, sp, sy, cr, cp, cy;
///     // static to help MS compiler fp bugs
///
///     angle = angles[YAW] * (M_PI*2 / 360);
///     sy = sin(angle);
///     cy = cos(angle);
///     angle = angles[PITCH] * (M_PI*2 / 360);
///     sp = sin(angle);
///     cp = cos(angle);
///     angle = angles[ROLL] * (M_PI*2 / 360);
///     sr = sin(angle);
///     cr = cos(angle);
///
///     if (forward)
///     {
///         forward[0] = cp*cy;
///         forward[1] = cp*sy;
///         forward[2] = -sp;
///     }
///     if (right)
///     {
///         right[0] = (-1*sr*sp*cy+-1*cr*-sy);
///         right[1] = (-1*sr*sp*sy+-1*cr*cy);
///         right[2] = -1*sr*cp;
///     }
///     if (up)
///     {
///         up[0] = (cr*sp*cy+-sr*-sy);
///         up[1] = (cr*sp*sy+-sr*cy);
///         up[2] = cr*cp;
///     }
/// }
/// ```
///
/// The six `static float` variables are always written before they are read, so
/// they behave exactly like locals; they are kept as locals here.
#[no_mangle]
pub unsafe extern "C" fn AngleVectors(
    angles: *const vec_t,
    forward: *mut vec_t,
    right: *mut vec_t,
    up: *mut vec_t,
) {
    let mut angle: f32;
    let sr: f32;
    let sp: f32;
    let sy: f32;
    let cr: f32;
    let cp: f32;
    let cy: f32;

    angle = (*angles.add(YAW) as f64 * (M_PI * 2.0 / 360.0)) as f32;
    sy = (angle as f64).sin() as f32;
    cy = (angle as f64).cos() as f32;
    angle = (*angles.add(PITCH) as f64 * (M_PI * 2.0 / 360.0)) as f32;
    sp = (angle as f64).sin() as f32;
    cp = (angle as f64).cos() as f32;
    angle = (*angles.add(ROLL) as f64 * (M_PI * 2.0 / 360.0)) as f32;
    sr = (angle as f64).sin() as f32;
    cr = (angle as f64).cos() as f32;

    // The nine expressions below are the C ones with two purely mechanical
    // rewrites, so that they are not just *value*-identical to what
    // `gcc -O0 -Iinc` emits but *bit*-identical for NaN operands too:
    //
    // 1. every `-1 * x` factor and every double negation is folded into a single
    //    sign flip, exactly as gcc folds them:
    //        -1*sr      -> -sr        -1*cr*-sy  ->  cr*sy
    //        -1*cr      -> -cr        -sr*-sy    ->  sr*sy
    //    Multiplying by -1 is an exact sign flip for every non-NaN value, so no
    //    result changes -- but it matters for NaN, because gcc's `xorps` flips a
    //    NaN's sign bit while `mulss` by -1.0 keeps it.
    // 2. the operands of the remaining `*` and `+` are written in the order gcc
    //    puts them in the `mulss`/`addss` *first source operand* (which is the
    //    one whose NaN payload survives when both operands are NaN).  Both
    //    operations are commutative for every non-NaN value, so again no result
    //    changes.  For instance `right[0] = (-1*sr*sp*cy+-1*cr*-sy)` becomes
    //    `cr*sy` first, because gcc computes it into the destination register of
    //    the `addss`.
    if !forward.is_null() {
        *forward.add(0) = cy * cp;
        *forward.add(1) = sy * cp;
        *forward.add(2) = -sp;
    }
    if !right.is_null() {
        *right.add(0) = sy * cr + -sr * sp * cy;
        *right.add(1) = cy * -cr + -sr * sp * sy;
        *right.add(2) = cp * -sr;
    }
    if !up.is_null() {
        *up.add(0) = sy * sr + cr * sp * cy;
        *up.add(1) = cy * -sr + cr * sp * sy;
        *up.add(2) = cp * cr;
    }
}

/// ```c
/// /*
/// ** assumes "src" is normalized
/// */
/// void PerpendicularVector( vec3_t dst, const vec3_t src )
/// {
///     int pos;
///     int i;
///     float minelem = 1.0F;
///     vec3_t tempvec;
///
///     /*
///     ** find the smallest magnitude axially aligned vector
///     */
///     for ( pos = 0, i = 0; i < 3; i++ )
///     {
///         if ( fabs( src[i] ) < minelem )
///         {
///             pos = i;
///             minelem = fabs( src[i] );
///         }
///     }
///     tempvec[0] = tempvec[1] = tempvec[2] = 0.0F;
///     tempvec[pos] = 1.0F;
///
///     /*
///     ** project the point onto the plane defined by src
///     */
///     ProjectPointOnPlane( dst, tempvec, src );
///
///     /*
///     ** normalize the result
///     */
///     VectorNormalize( dst );
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn PerpendicularVector(dst: *mut vec_t, src: *const vec_t) {
    let mut pos: usize;
    let mut minelem: f32 = 1.0f32;
    let mut tempvec: [vec_t; 3] = [0.0; 3];

    // find the smallest magnitude axially aligned vector
    pos = 0;
    for i in 0..3usize {
        if (*src.add(i) as f64).abs() < minelem as f64 {
            pos = i;
            minelem = (*src.add(i) as f64).abs() as f32;
        }
    }
    tempvec[2] = 0.0f32;
    tempvec[1] = tempvec[2];
    tempvec[0] = tempvec[1];
    tempvec[pos] = 1.0f32;

    // project the point onto the plane defined by src
    ProjectPointOnPlane(dst, tempvec.as_ptr(), src);

    // normalize the result
    VectorNormalize(dst);
}
