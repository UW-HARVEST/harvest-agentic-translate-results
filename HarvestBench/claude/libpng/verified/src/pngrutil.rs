//! Translation of `c_src/src/pngrutil.c`

use crate::*;

pub(crate) const LZ77Min: png_uint_32 = 2 + 5 + 4;

/* Arrays to facilitate interlacing - use pass (0 - 6) as index. */
pub(crate) static png_pass_start: [png_byte; 7] = [0, 4, 0, 2, 0, 1, 0];
pub(crate) static png_pass_inc: [png_byte; 7] = [8, 8, 4, 4, 2, 2, 1];
pub(crate) static png_pass_ystart: [png_byte; 7] = [0, 0, 4, 0, 2, 0, 1];
pub(crate) static png_pass_yinc: [png_byte; 7] = [8, 8, 8, 4, 4, 2, 2];


/* ---------------------------------------------------------------------- *
 * The table driven interface to all the chunk handling (pngrutil.c ~2986)
 * ---------------------------------------------------------------------- */

pub(crate) type png_index = c_int;

pub(crate) const NoCheck: png_uint_32 = 0x801;
pub(crate) const Limit: png_uint_32 = 0x802;
pub(crate) const LKMin: png_uint_32 = 3 + LZ77Min;

const hIHDR: png_uint_32 = PNG_HAVE_IHDR;
const hPLTE: png_uint_32 = PNG_HAVE_PLTE;
const hIDAT: png_uint_32 = PNG_HAVE_IDAT;
const hCOL: png_uint_32 = PNG_HAVE_PLTE | PNG_HAVE_IDAT;
const aIDAT: png_uint_32 = PNG_AFTER_IDAT;

pub(crate) type png_chunk_handler_fn =
    Option<unsafe extern "C" fn(png_structrp, png_inforp, png_uint_32) -> png_handle_result_code>;

#[derive(Clone, Copy)]
pub(crate) struct read_chunk_entry {
    pub handler: png_chunk_handler_fn,
    pub max_length: png_uint_32, /* :12 */
    pub min_length: png_uint_32, /* :8  */
    pub pos_before: png_uint_32, /* :4  */
    pub pos_after: png_uint_32,  /* :4  */
    pub multiple: png_uint_32,   /* :1  */
}

#[inline]
const fn RCE(
    handler: png_chunk_handler_fn,
    max_length: png_uint_32,
    min_length: png_uint_32,
    pos_before: png_uint_32,
    pos_after: png_uint_32,
    multiple: png_uint_32,
) -> read_chunk_entry {
    read_chunk_entry { handler, max_length, min_length, pos_before, pos_after, multiple }
}

pub(crate) static read_chunks: [read_chunk_entry; PNG_INDEX_unknown as usize] = [
    /*  0 IHDR */ RCE(Some(png_handle_IHDR),   13,     13,     hIHDR, 0,     0),
    /*  1 PLTE */ RCE(Some(png_handle_PLTE),   NoCheck, 0,     0,     hIHDR, 1),
    /*  2 IDAT */ RCE(None,                    NoCheck, 0,     aIDAT, hIHDR, 1),
    /*  3 IEND */ RCE(Some(png_handle_IEND),   NoCheck, 0,     0,     aIDAT, 0),
    /*  4 acTL */ RCE(None,                    8,       8,     hIDAT, hIHDR, 0),
    /*  5 bKGD */ RCE(Some(png_handle_bKGD),   6,       1,     hIDAT, hIHDR, 0),
    /*  6 cHRM */ RCE(Some(png_handle_cHRM),   32,      32,    hCOL,  hIHDR, 0),
    /*  7 cICP */ RCE(Some(png_handle_cICP),   4,       4,     hCOL,  hIHDR, 0),
    /*  8 cLLI */ RCE(Some(png_handle_cLLI),   8,       8,     hCOL,  hIHDR, 0),
    /*  9 eXIf */ RCE(Some(png_handle_eXIf),   Limit,   4,     0,     hIHDR, 0),
    /* 10 fcTL */ RCE(None,                    25,      26,    0,     hIHDR, 1),
    /* 11 fdAT */ RCE(None,                    Limit,   4,     hIDAT, hIHDR, 1),
    /* 12 gAMA */ RCE(Some(png_handle_gAMA),   4,       4,     hCOL,  hIHDR, 0),
    /* 13 hIST */ RCE(Some(png_handle_hIST),   1024,    0,     hPLTE, hIHDR, 0),
    /* 14 iCCP */ RCE(Some(png_handle_iCCP),   NoCheck, LKMin, hCOL,  hIHDR, 0),
    /* 15 iTXt */ RCE(Some(png_handle_iTXt),   NoCheck, 6,     0,     hIHDR, 1),
    /* 16 mDCV */ RCE(Some(png_handle_mDCV),   24,      24,    hCOL,  hIHDR, 0),
    /* 17 oFFs */ RCE(Some(png_handle_oFFs),   9,       9,     hIDAT, hIHDR, 0),
    /* 18 pCAL */ RCE(Some(png_handle_pCAL),   NoCheck, 14,    hIDAT, hIHDR, 0),
    /* 19 pHYs */ RCE(Some(png_handle_pHYs),   9,       9,     hIDAT, hIHDR, 0),
    /* 20 sBIT */ RCE(Some(png_handle_sBIT),   4,       1,     hCOL,  hIHDR, 0),
    /* 21 sCAL */ RCE(Some(png_handle_sCAL),   Limit,   4,     hIDAT, hIHDR, 0),
    /* 22 sPLT */ RCE(Some(png_handle_sPLT),   NoCheck, 3,     hIDAT, hIHDR, 1),
    /* 23 sRGB */ RCE(Some(png_handle_sRGB),   1,       1,     hCOL,  hIHDR, 0),
    /* 24 tEXt */ RCE(Some(png_handle_tEXt),   NoCheck, 2,     0,     hIHDR, 1),
    /* 25 tIME */ RCE(Some(png_handle_tIME),   7,       7,     0,     hIHDR, 0),
    /* 26 tRNS */ RCE(Some(png_handle_tRNS),   256,     0,     hIDAT, hIHDR, 0),
    /* 27 zTXt */ RCE(Some(png_handle_zTXt),   Limit,   LKMin, 0,     hIHDR, 1),
];

include!("gen/pngrutil_p01.rs");
include!("gen/pngrutil_p02.rs");
include!("gen/pngrutil_p03.rs");
include!("gen/pngrutil_p04.rs");
include!("gen/pngrutil_p05.rs");
include!("gen/pngrutil_p06.rs");
include!("gen/pngrutil_p07.rs");
include!("gen/pngrutil_p08.rs");
include!("gen/pngrutil_p09.rs");
include!("gen/pngrutil_p10.rs");
