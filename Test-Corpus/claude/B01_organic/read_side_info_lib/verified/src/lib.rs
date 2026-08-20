//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exposing
//! exactly one public symbol, `read_side_info` (see `include/lib.h`). There are
//! no namespace/renaming macros in the public header, so the linker symbol name
//! is identical to the source-level name.
//!
//! Behaviour is reproduced exactly, including the quirks of the original code:
//!   * `get_bits` advances `bs->pos` *before* the limit check and still returns
//!     0 (without reading) when the limit is exceeded.
//!   * `get_bits` reads bytes via raw pointer arithmetic and may read past the
//!     end of the caller's buffer, just like the C.
//!   * `region_count[2]` is deliberately left untouched on the short-block
//!     path (the C never assigns it there), so the caller's previous value
//!     survives.
//!   * All arithmetic wraps/truncates exactly as the C integer conversions do.

#![allow(non_camel_case_types)]

use std::ffi::c_int;

/// Mirrors the C `bs_t`:
///
/// ```c
/// typedef struct {
///     const uint8_t *buf;
///     int pos, limit;
/// } bs_t;
/// ```
#[repr(C)]
pub struct bs_t {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

/// Mirrors the C `L3_gr_info_t`:
///
/// ```c
/// typedef struct {
///     const uint8_t *sfbtab;
///     uint16_t part_23_length, big_values, scalefac_compress;
///     uint8_t global_gain, block_type, mixed_block_flag, n_long_sfb, n_short_sfb;
///     uint8_t table_select[3], region_count[3], subblock_gain[3];
///     uint8_t preflag, scalefac_scale, count1_table, scfsi;
/// } L3_gr_info_t;
/// ```
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

/// The three scalefactor-band tables that the C declares as function-local
/// `static const uint8_t` arrays.
///
/// `sr_idx` is derived from the header and can reach 8 (sample-rate index 3
/// combined with the MPEG1 / not-MPEG2.5 version bits), which indexes one row
/// *past* the end of every `[8][...]` table. The C therefore performs an
/// out-of-bounds read whose result depends purely on how the compiler happened
/// to lay the tables out in `.rodata` — it is not stable even between two
/// builds of the C itself (gcc `-O2` reverses the order relative to gcc `-O0`).
///
/// The reference build (`c_src/CMakeLists.txt`) sets no `CMAKE_BUILD_TYPE`, so
/// it compiles at `-O0`, where gcc — and clang at any level — emits the tables
/// in declaration order:
///
/// ```text
///   g_scf_long   +0     184 bytes
///   (padding)    +184     8 bytes of zeros
///   g_scf_short  +192   320 bytes
///   g_scf_mixed  +512   320 bytes   (.rodata ends at +832)
/// ```
///
/// This `#[repr(C)]` blob reproduces that layout byte for byte (all fields have
/// alignment 1, so the offsets above are exact). Consequently row 8 of `long`
/// yields the 8 zero pad bytes followed by the start of `short`'s row 0, and
/// row 8 of `short` aliases `mixed`'s row 0 — matching the C exactly. Row 8 of
/// `mixed` runs off the end of `.rodata` in the C as well (into `.eh_frame_hdr`,
/// i.e. build-specific unwind data that no two builds agree on); `tail` supplies
/// deterministic zero bytes there so this translation can never fault.
#[repr(C)]
struct ScfTables {
    long_: [[u8; 23]; 8],
    pad: [u8; 8],
    short_: [[u8; 40]; 8],
    mixed: [[u8; 40]; 8],
    tail: [u8; 64],
}

static G_SCF: ScfTables = ScfTables {
    // 8 bytes of alignment padding that gcc/clang insert after the 184-byte
    // g_scf_long, before g_scf_short. Part of what row 8 of `long` reads.
    pad: [0; 8],
    // static const uint8_t g_scf_mixed[8][40]
    // Note: several rows have fewer than 40 initialisers in the C and are
    // therefore zero-filled up to 40 elements.
    mixed: [
        [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24,
            24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0,
        ],
        [
            12, 12, 12, 4, 4, 4, 8, 8, 8, 12, 12, 12, 16, 16, 16, 20, 20, 20, 24, 24, 24, 28, 28,
            28, 36, 36, 36, 2, 2, 2, 2, 2, 2, 2, 2, 2, 26, 26, 26, 0,
        ],
        [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 14, 14, 14, 18, 18, 18, 26,
            26, 26, 32, 32, 32, 42, 42, 42, 18, 18, 18, 0, 0, 0, 0,
        ],
        [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24,
            24, 24, 32, 32, 32, 44, 44, 44, 12, 12, 12, 0, 0, 0, 0,
        ],
        [
            6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24,
            24, 24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0,
        ],
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
    // static const uint8_t g_scf_short[8][40]
    short_: [
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
    // static const uint8_t g_scf_long[8][23]
    long_: [
        [
            6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0,
        ],
        [
            12, 12, 12, 12, 12, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 76, 90, 2, 2, 2, 2, 2, 0,
        ],
        [
            6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0,
        ],
        [
            6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 18, 22, 26, 32, 38, 46, 54, 62, 70, 76, 36, 0,
        ],
        [
            6, 6, 6, 6, 6, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 38, 46, 52, 60, 68, 58, 54, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 6, 6, 8, 8, 10, 12, 16, 20, 24, 28, 34, 42, 50, 54, 76, 158, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 10, 12, 16, 18, 22, 28, 34, 40, 46, 54, 54, 192, 0,
        ],
        [
            4, 4, 4, 4, 4, 4, 6, 6, 8, 10, 12, 16, 20, 24, 30, 38, 46, 56, 68, 84, 102, 26, 0,
        ],
    ],
    tail: [0; 64],
};

// Lock in the layout the C relies on (see `ScfTables` docs), and the ABI of the
// two public structs, so any accidental change becomes a compile error.
const _: () = {
    assert!(core::mem::offset_of!(ScfTables, long_) == 0);
    assert!(core::mem::offset_of!(ScfTables, pad) == 184);
    assert!(core::mem::offset_of!(ScfTables, short_) == 192);
    assert!(core::mem::offset_of!(ScfTables, mixed) == 512);
    assert!(core::mem::offset_of!(ScfTables, tail) == 832);

    assert!(core::mem::size_of::<bs_t>() == 16);
    assert!(core::mem::offset_of!(bs_t, pos) == 8);
    assert!(core::mem::offset_of!(bs_t, limit) == 12);

    assert!(core::mem::size_of::<L3_gr_info_t>() == 32);
    assert!(core::mem::offset_of!(L3_gr_info_t, part_23_length) == 8);
    assert!(core::mem::offset_of!(L3_gr_info_t, big_values) == 10);
    assert!(core::mem::offset_of!(L3_gr_info_t, scalefac_compress) == 12);
    assert!(core::mem::offset_of!(L3_gr_info_t, global_gain) == 14);
    assert!(core::mem::offset_of!(L3_gr_info_t, table_select) == 19);
    assert!(core::mem::offset_of!(L3_gr_info_t, region_count) == 22);
    assert!(core::mem::offset_of!(L3_gr_info_t, subblock_gain) == 25);
    assert!(core::mem::offset_of!(L3_gr_info_t, preflag) == 28);
    assert!(core::mem::offset_of!(L3_gr_info_t, scfsi) == 31);
};

/// `&g_scf_long[idx]` — unchecked row addressing, matching the C.
#[inline]
fn scf_long_row(idx: c_int) -> *const u8 {
    unsafe { (G_SCF.long_.as_ptr() as *const u8).offset((idx as isize) * 23) }
}

/// `&g_scf_short[idx]` — unchecked row addressing, matching the C.
#[inline]
fn scf_short_row(idx: c_int) -> *const u8 {
    unsafe { (G_SCF.short_.as_ptr() as *const u8).offset((idx as isize) * 40) }
}

/// `&g_scf_mixed[idx]` — unchecked row addressing, matching the C.
#[inline]
fn scf_mixed_row(idx: c_int) -> *const u8 {
    unsafe { (G_SCF.mixed.as_ptr() as *const u8).offset((idx as isize) * 40) }
}

/// Translation of the file-local C helper:
///
/// ```c
/// static uint32_t get_bits(bs_t *bs, int n) {
///     uint32_t next, cache = 0, s = bs->pos & 7;
///     int shl = n + s;
///     const uint8_t *p = bs->buf + (bs->pos >> 3);
///     if ((bs->pos += n) > bs->limit)
///         return 0;
///     next = *p++ & (255 >> s);
///     while ((shl -= 8) > 0) {
///         cache |= next << shl;
///         next = *p++;
///     }
///     return cache | (next >> -shl);
/// }
/// ```
///
/// `static` in C means this symbol is not exported, so it stays private here.
/// Note that `bs->pos` is advanced even when the limit check fails.
unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    unsafe {
        let mut next: u32;
        let mut cache: u32 = 0;
        // `s` is `uint32_t` in the C; `pos & 7` is always in 0..=7 (also for
        // negative `pos` under two's complement, which is what GCC produces).
        let s: u32 = ((*bs).pos & 7) as u32;
        // `int shl = n + s;` — `n` is converted to unsigned, the sum computed in
        // unsigned, then converted back to int.
        let mut shl: c_int = (n as u32).wrapping_add(s) as c_int;
        let mut p: *const u8 = (*bs).buf.offset(((*bs).pos >> 3) as isize);

        (*bs).pos = (*bs).pos.wrapping_add(n);
        if (*bs).pos > (*bs).limit {
            return 0;
        }

        next = (*p & (255u32 >> s) as u8) as u32;
        p = p.add(1);

        loop {
            shl = shl.wrapping_sub(8);
            if shl <= 0 {
                break;
            }
            cache |= next.wrapping_shl(shl as u32);
            next = *p as u32;
            p = p.add(1);
        }

        // `shl` is now in -7..=0, so `-shl` is a well-defined 0..=7 shift.
        cache | next.wrapping_shr(shl.wrapping_neg() as u32)
    }
}

/// Translation of the sole public C entry point:
///
/// ```c
/// int read_side_info(bs_t *bs, L3_gr_info_t *gr, const uint8_t *hdr);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn read_side_info(
    bs: *mut bs_t,
    gr: *mut L3_gr_info_t,
    hdr: *const u8,
) -> c_int {
    unsafe {
        let mut gr = gr;

        let mut tables: u32;
        // `unsigned scfsi = 0;`
        let mut scfsi: u32 = 0;
        // `int main_data_begin` is uninitialised in the C but is unconditionally
        // assigned by both arms of the branch below before any use.
        let main_data_begin: c_int;
        let mut part_23_sum: c_int = 0;

        let hdr1 = *hdr.add(1) as c_int;
        let hdr2 = *hdr.add(2) as c_int;
        let hdr3 = *hdr.add(3) as c_int;

        let mut sr_idx: c_int = ((hdr2 >> 2) & 3) + (((hdr1 >> 3) & 1) + ((hdr1 >> 4) & 1)) * 3;
        sr_idx -= (sr_idx != 0) as c_int;

        let mut gr_count: c_int = if (hdr3 & 0xC0) == 0xC0 { 1 } else { 2 };

        if (hdr1 & 0x8) != 0 {
            gr_count *= 2;
            main_data_begin = get_bits(bs, 9) as c_int;
            scfsi = get_bits(bs, 7 + gr_count);
        } else {
            main_data_begin = (get_bits(bs, 8 + gr_count) >> gr_count) as c_int;
        }

        loop {
            if (hdr3 & 0xC0) == 0xC0 {
                scfsi = scfsi.wrapping_shl(4);
            }

            (*gr).part_23_length = get_bits(bs, 12) as u16;
            part_23_sum = part_23_sum.wrapping_add((*gr).part_23_length as c_int);
            (*gr).big_values = get_bits(bs, 9) as u16;
            if (*gr).big_values > 288 {
                return -1;
            }
            (*gr).global_gain = get_bits(bs, 8) as u8;
            (*gr).scalefac_compress = get_bits(bs, if (hdr1 & 0x8) != 0 { 4 } else { 9 }) as u16;
            (*gr).sfbtab = scf_long_row(sr_idx);
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
                        (*gr).sfbtab = scf_short_row(sr_idx);
                        (*gr).n_long_sfb = 0;
                        (*gr).n_short_sfb = 39;
                    } else {
                        (*gr).sfbtab = scf_mixed_row(sr_idx);
                        (*gr).n_long_sfb = if (hdr1 & 0x8) != 0 { 8 } else { 6 };
                        (*gr).n_short_sfb = 30;
                    }
                }
                tables = get_bits(bs, 10);
                tables = tables.wrapping_shl(5);
                (*gr).subblock_gain[0] = get_bits(bs, 3) as u8;
                (*gr).subblock_gain[1] = get_bits(bs, 3) as u8;
                (*gr).subblock_gain[2] = get_bits(bs, 3) as u8;
                // NOTE: the C does not assign region_count[2] on this path.
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
            (*gr).preflag = if (hdr1 & 0x8) != 0 {
                get_bits(bs, 1) as u8
            } else {
                ((*gr).scalefac_compress >= 500) as u8
            };
            (*gr).scalefac_scale = get_bits(bs, 1) as u8;
            (*gr).count1_table = get_bits(bs, 1) as u8;
            (*gr).scfsi = ((scfsi >> 12) & 15) as u8;
            scfsi = scfsi.wrapping_shl(4);
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
