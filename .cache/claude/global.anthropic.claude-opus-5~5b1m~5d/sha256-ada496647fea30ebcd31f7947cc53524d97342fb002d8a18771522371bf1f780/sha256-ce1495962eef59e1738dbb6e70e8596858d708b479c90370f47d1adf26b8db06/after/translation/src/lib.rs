//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` of the C shared object exactly):
//!   * `read_side_info`
//!
//! The translation is intentionally literal: evaluation order, integer widths,
//! truncating casts, the order of validation/error checks and even the quirks of
//! the original code are reproduced rather than "fixed".

#![allow(non_camel_case_types)]

use core::ffi::c_int;

/// `typedef struct { const uint8_t *buf; int pos, limit; } bs_t;`
/// (size 16, align 8 — verified against the C compiler.)
#[repr(C)]
pub struct bs_t {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

/// `L3_gr_info_t` — size 32, align 8, field offsets verified against the C
/// compiler (sfbtab 0, part_23_length 8, big_values 10, scalefac_compress 12,
/// global_gain 14, block_type 15, mixed_block_flag 16, n_long_sfb 17,
/// n_short_sfb 18, table_select 19, region_count 22, subblock_gain 25,
/// preflag 28, scalefac_scale 29, count1_table 30, scfsi 31).
#[repr(C)]
pub struct L3_gr_info_t {
    pub sfbtab: *const u8,
    pub part_23_length: u16,
    pub big_values: u16,
    pub scalefac_compress: u16,
    pub global_gain: u8,
    pub block_type: u8,
    pub mixed_block_flag: u8,
    pub n_long_sfb: u8,
    pub n_short_sfb: u8,
    pub table_select: [u8; 3],
    pub region_count: [u8; 3],
    pub subblock_gain: [u8; 3],
    pub preflag: u8,
    pub scalefac_scale: u8,
    pub count1_table: u8,
    pub scfsi: u8,
}

// ---------------------------------------------------------------------------
// Scalefactor band tables.
//
// In C these are three function-local `static const uint8_t` arrays:
//   g_scf_long[8][23], g_scf_short[8][40], g_scf_mixed[8][40]
//
// `sr_idx` can reach 8 here (an invalid sample-rate index of 3 combined with
// both MPEG version bits set gives a raw sum of 9, minus 1 => 8), so
// `g_scf_*[sr_idx]` may address one row PAST the end of a table. Rows 0..=7 --
// every in-bounds case -- must of course be byte-identical, but the row-8 case
// is observable too: the returned `sfbtab` pointer aliases whatever the
// compiler placed after the indexed array. To reproduce that we must reproduce
// the C object's `.rodata` LAYOUT, not just the table contents.
//
// The layout below is the one the C build actually emits, read straight off the
// shared object (`nm` gives the addresses, `objdump -s -j .rodata` the bytes):
//
//   g_scf_long.2   0x2000   offset   0, 8 * 23 = 184 bytes
//   (padding)               offset 184, 8 zero bytes -- gcc aligns the
//                           following array to a 32-byte boundary
//   g_scf_short.1  0x20c0   offset 192, 8 * 40 = 320 bytes
//   g_scf_mixed.0  0x2200   offset 512, 8 * 40 = 320 bytes
//   .eh_frame_hdr  0x2340   offset 832 -- end of .rodata
//
// So in C `g_scf_long[8]` aliases the 8 pad bytes followed by the start of
// `g_scf_short[0]`, and `g_scf_short[8]` aliases `g_scf_mixed[0]` exactly. The
// `ScfTables` struct below is `#[repr(C)]` with those very fields, so it
// reproduces both aliases; `TRAILING_PAD` keeps the `g_scf_mixed[8]` case
// inside our own allocation instead of being UB on the Rust side. (In C
// `g_scf_mixed[8]` reads past the end of `.rodata` into linker-generated
// `.eh_frame_hdr` bytes, which are not library data and are not reproducible
// in any Rust object -- see CONFIGS.md row C17.)
//
// Rows whose C initializer is shorter than the declared row length are
// zero-filled, which is what C does; `g_scf_mixed` rows 0/2/3/4 have 37
// initializers and rows 5/6/7 have 39, so they are written out in full here.
//
// `tests/differential.rs::layout_matches_c_rodata` re-derives all of this from
// the C `.so` at test time and asserts byte-for-byte equality, so this cannot
// silently drift.
// ---------------------------------------------------------------------------

/// Mirrors the `.rodata` layout of the C translation unit. `#[repr(C)]` fixes
/// the field order and the 32-byte alignment reproduces gcc's padding.
#[repr(C, align(32))]
struct ScfTables {
    g_scf_long: [[u8; 23]; 8],
    _pad: [u8; 8],
    g_scf_short: [[u8; 40]; 8],
    g_scf_mixed: [[u8; 40]; 8],
    _trailing_pad: [u8; TRAILING_PAD],
}

/// Enough slack for the one-row-past-the-end `g_scf_mixed[8]` read to stay
/// inside our own allocation.
const TRAILING_PAD: usize = 64;

const OFF_LONG: usize = 0;
const OFF_SHORT: usize = 192;
const OFF_MIXED: usize = 512;

// Compile-time proof that the struct really has the C object's layout.
const _: () = {
    assert!(core::mem::offset_of!(ScfTables, g_scf_long) == OFF_LONG);
    assert!(core::mem::offset_of!(ScfTables, g_scf_short) == OFF_SHORT);
    assert!(core::mem::offset_of!(ScfTables, g_scf_mixed) == OFF_MIXED);
    // g_scf_long[8] must start in the pad, 184 bytes in.
    assert!(OFF_LONG + 8 * 23 == 184);
    // g_scf_short[8] must coincide exactly with g_scf_mixed[0].
    assert!(OFF_SHORT + 8 * 40 == OFF_MIXED);
    // g_scf_mixed[8] must still be inside the allocation.
    assert!(OFF_MIXED + 8 * 40 + 40 <= core::mem::size_of::<ScfTables>());
};

static G_SCF_TABLES: ScfTables = ScfTables {
    g_scf_long: [
        [6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0],
        [12, 12, 12, 12, 12, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 76, 90, 2, 2, 2, 2, 2, 0],
        [6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0],
        [6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 18, 22, 26, 32, 38, 46, 54, 62, 70, 76, 36, 0],
        [6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0],
        [4, 4, 4, 4, 4, 4, 6, 6, 8, 8, 10, 12, 16, 20, 24, 28, 34, 42, 50, 54, 76, 158, 0],
        [4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 10, 12, 16, 18, 22, 28, 34, 40, 46, 54, 54, 192, 0],
        [4, 4, 4, 4, 4, 4, 6, 6, 8, 10, 12, 16, 20, 24, 30, 38, 46, 56, 68, 84, 102, 26, 0],
    ],
    _pad: [0; 8],
    g_scf_short: [
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18,
            18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0,
        ],
        [
            8, 8, 8, 8, 8, 8, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 24, 24, 24, 28, 28, 28,
            36, 36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 14, 14, 14, 18, 18,
            18, 26, 26, 26, 32, 32, 32, 42, 42, 42, 18, 18, 18, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18,
            18, 18, 24, 24, 24, 32, 32, 32, 44, 44, 44, 12, 12, 12, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18,
            18, 18, 24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14,
            14, 18, 18, 18, 22, 22, 22, 30, 30, 30, 56, 56, 56, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 6, 6, 6, 10, 10, 10, 12, 12, 12, 14, 14,
            14, 16, 16, 16, 20, 20, 20, 26, 26, 26, 66, 66, 66, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20,
            20, 26, 26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 12, 0,
        ],
    ],
    g_scf_mixed: [
        // 37 initializers in C -> last 3 bytes zero-filled.
        [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18,
            24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0,
        ],
        // 40 initializers in C -> no zero fill.
        [
            12, 12, 12, 4, 4, 4, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 24, 24, 24, 28, 28,
            28, 36, 36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26, 0,
        ],
        [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 14, 14, 14, 18, 18, 18, 26,
            26, 26, 32, 32, 32, 42, 42, 42, 18, 18, 18, 0, 0, 0, 0,
        ],
        [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18,
            24, 24, 24, 32, 32, 32, 44, 44, 44, 12, 12, 12, 0, 0, 0, 0,
        ],
        [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18,
            24, 24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0,
        ],
        // 39 initializers in C -> last byte zero-filled.
        [
            4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14,
            18, 18, 18, 22, 22, 22, 30, 30, 30, 56, 56, 56, 0, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 6, 6, 6, 10, 10, 10, 12, 12, 12, 14, 14, 14,
            16, 16, 16, 20, 20, 20, 26, 26, 26, 66, 66, 66, 0, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 6, 6, 6, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20,
            26, 26, 26, 34, 34, 34, 42, 42, 42, 12, 12, 12, 0, 0,
        ],
    ],
    _trailing_pad: [0; TRAILING_PAD],
};

/// `static uint32_t get_bits(bs_t *bs, int n)`
///
/// Note the original advances `bs->pos` by `n` *before* the overrun test and
/// returns 0 without reading the buffer when the limit is exceeded; the
/// position stays advanced. Shifts use wrapping semantics to match the
/// x86 shift instructions the C compiler emits (C itself is UB there).
#[inline]
unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    unsafe {
        let next: u32;
        let mut cache: u32 = 0;
        let s: u32 = ((*bs).pos & 7) as u32;
        let mut shl: c_int = (n as u32).wrapping_add(s) as c_int;
        let mut p: *const u8 = (*bs).buf.offset(((*bs).pos >> 3) as isize);

        (*bs).pos = (*bs).pos.wrapping_add(n);
        if (*bs).pos > (*bs).limit {
            return 0;
        }

        let mut nxt: u32 = (*p as u32) & (255u32 >> s);
        p = p.add(1);
        loop {
            shl = shl.wrapping_sub(8);
            if shl <= 0 {
                break;
            }
            cache |= nxt.wrapping_shl(shl as u32);
            nxt = *p as u32;
            p = p.add(1);
        }
        next = nxt;
        cache | next.wrapping_shr(shl.wrapping_neg() as u32)
    }
}

/// `int read_side_info(bs_t *bs, L3_gr_info_t *gr, const uint8_t *hdr)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn read_side_info(
    bs: *mut bs_t,
    gr: *mut L3_gr_info_t,
    hdr: *const u8,
) -> c_int {
    unsafe {
        let tables_base: *const u8 = (&raw const G_SCF_TABLES) as *const u8;

        let mut tables: u32;
        let mut scfsi: u32 = 0;
        let main_data_begin: c_int;
        let mut part_23_sum: c_int = 0;

        // `hdr` is re-read at every point the C dereferences it rather than
        // being hoisted into locals. The C accesses `hdr[1]` and `hdr[3]` from
        // *inside* the granule loop, interleaved with writes through `gr`, and
        // the reference build carries no `-O` flag so every access is a fresh
        // load. Hoisting would change the result if a caller passed a `hdr`
        // that aliases the `gr` array.
        macro_rules! hb {
            ($i:expr) => {
                *hdr.add($i) as c_int
            };
        }

        let mut sr_idx: c_int =
            ((hb!(2) >> 2) & 3) + (((hb!(1) >> 3) & 1) + ((hb!(1) >> 4) & 1)) * 3;
        sr_idx -= (sr_idx != 0) as c_int;

        let mut gr_count: c_int = if (hb!(3) & 0xC0) == 0xC0 { 1 } else { 2 };

        if (hb!(1) & 0x8) != 0 {
            gr_count *= 2;
            main_data_begin = get_bits(bs, 9) as c_int;
            scfsi = get_bits(bs, 7 + gr_count);
        } else {
            main_data_begin =
                get_bits(bs, 8 + gr_count).wrapping_shr(gr_count as u32) as c_int;
        }

        let mut gr: *mut L3_gr_info_t = gr;
        loop {
            if (hb!(3) & 0xC0) == 0xC0 {
                scfsi <<= 4;
            }
            (*gr).part_23_length = get_bits(bs, 12) as u16;
            part_23_sum = part_23_sum.wrapping_add((*gr).part_23_length as c_int);
            (*gr).big_values = get_bits(bs, 9) as u16;
            if (*gr).big_values > 288 {
                return -1;
            }
            (*gr).global_gain = get_bits(bs, 8) as u8;
            (*gr).scalefac_compress =
                get_bits(bs, if (hb!(1) & 0x8) != 0 { 4 } else { 9 }) as u16;
            (*gr).sfbtab = tables_base.add(OFF_LONG + (sr_idx as usize) * 23);
            (*gr).n_long_sfb = 22;
            (*gr).n_short_sfb = 0;
            if get_bits(bs, 1) != 0 {
                (*gr).block_type = get_bits(bs, 2) as u8;
                if (*gr).block_type == 0 {
                    return -1;
                }
                (*gr).mixed_block_flag = get_bits(bs, 1) as u8;
                (*gr).region_count[0] = 7;
                (*gr).region_count[1] = 255;
                if (*gr).block_type == 2 {
                    scfsi &= 0x0F0F;
                    if (*gr).mixed_block_flag == 0 {
                        (*gr).region_count[0] = 8;
                        (*gr).sfbtab =
                            tables_base.add(OFF_SHORT + (sr_idx as usize) * 40);
                        (*gr).n_long_sfb = 0;
                        (*gr).n_short_sfb = 39;
                    } else {
                        (*gr).sfbtab =
                            tables_base.add(OFF_MIXED + (sr_idx as usize) * 40);
                        (*gr).n_long_sfb = if (hb!(1) & 0x8) != 0 { 8 } else { 6 };
                        (*gr).n_short_sfb = 30;
                    }
                }
                tables = get_bits(bs, 10);
                tables <<= 5;
                (*gr).subblock_gain[0] = get_bits(bs, 3) as u8;
                (*gr).subblock_gain[1] = get_bits(bs, 3) as u8;
                (*gr).subblock_gain[2] = get_bits(bs, 3) as u8;
            } else {
                (*gr).block_type = 0;
                (*gr).mixed_block_flag = 0;
                tables = get_bits(bs, 15);
                (*gr).region_count[0] = get_bits(bs, 4) as u8;
                (*gr).region_count[1] = get_bits(bs, 3) as u8;
                (*gr).region_count[2] = 255;
            }
            (*gr).table_select[0] = (tables >> 10) as u8;
            (*gr).table_select[1] = ((tables >> 5) & 31) as u8;
            (*gr).table_select[2] = (tables & 31) as u8;
            (*gr).preflag = if (hb!(1) & 0x8) != 0 {
                get_bits(bs, 1) as u8
            } else {
                ((*gr).scalefac_compress >= 500) as u8
            };
            (*gr).scalefac_scale = get_bits(bs, 1) as u8;
            (*gr).count1_table = get_bits(bs, 1) as u8;
            (*gr).scfsi = ((scfsi >> 12) & 15) as u8;
            scfsi <<= 4;
            gr = gr.add(1);

            gr_count = gr_count.wrapping_sub(1);
            if gr_count == 0 {
                break;
            }
        }

        if part_23_sum.wrapping_add((*bs).pos)
            > (*bs).limit.wrapping_add(main_data_begin.wrapping_mul(8))
        {
            return -1;
        }
        main_data_begin
    }
}
