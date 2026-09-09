//! A translation of libpng 1.6.59.git to Rust.
//!
//! The public ABI (every symbol exported by the reference C shared library) is
//! reproduced exactly; behaviour, including error messages, error ordering and
//! byte-for-byte output, follows the C sources.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_imports)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(unused_labels)]
#![allow(unpredictable_function_pointer_comparisons)]
#![allow(clippy::all)]

pub mod cabi;
pub mod pngpriv;
pub mod pngstruct;
pub mod shared;
pub mod srgb_tables;
pub mod types;
pub mod zlib;

pub mod png_a;
pub mod png_b;
pub mod png_c;
pub mod png_d;
pub mod pngerror;
pub mod pngget;
pub mod pngmem;
pub mod pngpread;
pub mod pngread_a;
pub mod pngread_b;
pub mod pngread_c;
pub mod pngread_d;
pub mod pngrio;
pub mod pngrtran_a;
pub mod pngrtran_b;
pub mod pngrtran_c;
pub mod pngrtran_d;
pub mod pngrtran_e;
pub mod pngrutil_a;
pub mod pngrutil_b;
pub mod pngrutil_c;
pub mod pngrutil_d;
pub mod pngrutil_e;
pub mod pngset_a;
pub mod pngset_b;
pub mod pngtrans;
pub mod pngwio;
pub mod pngwrite_a;
pub mod pngwrite_b;
pub mod pngwtran;
pub mod pngwutil_a;
pub mod pngwutil_b;
pub mod pngwutil_c;

/// Items every translated module needs.
pub mod prelude {
    pub use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

    pub use crate::cabi::{memcmp, memcpy, memmove, memset, strcmp, strlen};
    pub use crate::pngpriv::*;
    pub use crate::pngstruct::*;
    pub use crate::shared::*;
    pub use crate::srgb_tables::{png_sRGB_base, png_sRGB_delta, png_sRGB_table};
    pub use crate::types::*;
    pub use crate::zlib;
    pub use crate::zlib::{
        adler32, crc32, deflate, deflateBound, deflateEnd, deflateInit2, deflateReset, inflate,
        inflateEnd, inflateInit, inflateInit2, inflateReset, inflateReset2, inflateValidate, uInt,
        uLong, z_stream, Z_BUF_ERROR, Z_DATA_ERROR, Z_DEFAULT_COMPRESSION, Z_DEFAULT_STRATEGY,
        Z_DEFLATED, Z_ERRNO, Z_FINISH, Z_MEM_ERROR, Z_NEED_DICT, Z_NO_FLUSH, Z_OK, Z_STREAM_END,
        Z_STREAM_ERROR, Z_SYNC_FLUSH, Z_VERSION_ERROR,
    };

    // Cross-module entry points (public *and* file-local helpers).
    pub use crate::png_a::*;
    pub use crate::png_b::*;
    pub use crate::png_c::*;
    pub use crate::png_d::*;
    pub use crate::pngerror::*;
    pub use crate::pngget::*;
    pub use crate::pngmem::*;
    pub use crate::pngpread::*;
    pub use crate::pngread_a::*;
    pub use crate::pngread_b::*;
    pub use crate::pngread_c::*;
    pub use crate::pngread_d::*;
    pub use crate::pngrio::*;
    pub use crate::pngrtran_a::*;
    pub use crate::pngrtran_b::*;
    pub use crate::pngrtran_c::*;
    pub use crate::pngrtran_d::*;
    pub use crate::pngrtran_e::*;
    pub use crate::pngrutil_a::*;
    pub use crate::pngrutil_b::*;
    pub use crate::pngrutil_c::*;
    pub use crate::pngrutil_d::*;
    pub use crate::pngrutil_e::*;
    pub use crate::pngset_a::*;
    pub use crate::pngset_b::*;
    pub use crate::pngtrans::*;
    pub use crate::pngwio::*;
    pub use crate::pngwrite_a::*;
    pub use crate::pngwrite_b::*;
    pub use crate::pngwtran::*;
    pub use crate::pngwutil_a::*;
    pub use crate::pngwutil_b::*;
    pub use crate::pngwutil_c::*;
}
