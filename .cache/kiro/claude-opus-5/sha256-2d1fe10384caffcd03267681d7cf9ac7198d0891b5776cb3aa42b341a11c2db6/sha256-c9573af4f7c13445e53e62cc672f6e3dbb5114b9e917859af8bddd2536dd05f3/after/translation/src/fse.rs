//! Translation of `common/fse.h` — constants, table layouts and the inlined
//! state machine helpers.
#![allow(dead_code)]

use core::ffi::{c_char, c_uint};

use crate::bitstream::*;
use crate::error::*;
use crate::mem::*;

pub const FSE_VERSION_MAJOR: u32 = 0;
pub const FSE_VERSION_MINOR: u32 = 9;
pub const FSE_VERSION_RELEASE: u32 = 0;
pub const FSE_VERSION_NUMBER: u32 =
    FSE_VERSION_MAJOR * 100 * 100 + FSE_VERSION_MINOR * 100 + FSE_VERSION_RELEASE;

pub const FSE_MAX_MEMORY_USAGE: u32 = 14;
pub const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
pub const FSE_MAX_SYMBOL_VALUE: u32 = 255;
pub const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2;
pub const FSE_MAX_TABLESIZE: u32 = 1 << FSE_MAX_TABLELOG;
pub const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
pub const FSE_MIN_TABLELOG: u32 = 5;
pub const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

/// `FSE_NCOUNTBOUND`
pub const FSE_NCOUNTBOUND: usize = 512;

/// `FSE_TABLESTEP(tableSize)`
#[inline(always)]
pub const fn fse_tablestep(table_size: usize) -> usize {
    (table_size >> 1) + (table_size >> 3) + 3
}

/// `FSE_DTable` — the C type is `unsigned`.
pub type FSE_DTable = U32;
/// `FSE_CTable` — the C type is `unsigned`.
pub type FSE_CTable = U32;

/// `FSE_BLOCKBOUND(size)`
#[inline(always)]
pub const fn fse_blockbound(size: usize) -> usize {
    size + (size >> 7) + 4 + core::mem::size_of::<usize>()
}

/// `FSE_COMPRESSBOUND(size)`
#[inline(always)]
pub const fn fse_compressbound(size: usize) -> usize {
    FSE_NCOUNTBOUND + fse_blockbound(size)
}

/// `FSE_CTABLE_SIZE_U32()`
#[inline(always)]
pub const fn fse_ctable_size_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    (1 + (1 << (max_table_log - 1)) + ((max_symbol_value + 1) * 2)) as usize
}

/// `FSE_DTABLE_SIZE_U32()`
#[inline(always)]
pub const fn fse_dtable_size_u32(max_table_log: u32) -> usize {
    (1 + (1u32 << max_table_log)) as usize
}

/// `FSE_DTABLE_SIZE()`
#[inline(always)]
pub const fn fse_dtable_size(max_table_log: u32) -> usize {
    fse_dtable_size_u32(max_table_log) * core::mem::size_of::<FSE_DTable>()
}

/// `FSE_BUILD_DTABLE_WKSP_SIZE()`
#[inline(always)]
pub const fn fse_build_dtable_wksp_size(max_table_log: u32, max_symbol_value: u32) -> usize {
    core::mem::size_of::<i16>() * (max_symbol_value as usize + 1)
        + (1usize << max_table_log)
        + 8
}

/// `FSE_BUILD_DTABLE_WKSP_SIZE_U32()`
#[inline(always)]
pub const fn fse_build_dtable_wksp_size_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    (fse_build_dtable_wksp_size(max_table_log, max_symbol_value)
        + core::mem::size_of::<c_uint>()
        - 1)
        / core::mem::size_of::<c_uint>()
}

/// `FSE_DECOMPRESS_WKSP_SIZE_U32()`
#[inline(always)]
pub const fn fse_decompress_wksp_size_u32(max_table_log: u32, max_symbol_value: u32) -> usize {
    fse_dtable_size_u32(max_table_log)
        + 1
        + fse_build_dtable_wksp_size_u32(max_table_log, max_symbol_value)
        + (FSE_MAX_SYMBOL_VALUE as usize + 1) / 2
        + 1
}

/// `FSE_DECOMPRESS_WKSP_SIZE()`
#[inline(always)]
pub const fn fse_decompress_wksp_size(max_table_log: u32, max_symbol_value: u32) -> usize {
    fse_decompress_wksp_size_u32(max_table_log, max_symbol_value)
        * core::mem::size_of::<c_uint>()
}

/// `FSE_repeat`
pub const FSE_repeat_none: u32 = 0;
pub const FSE_repeat_check: u32 = 1;
pub const FSE_repeat_valid: u32 = 2;

/* ============ compression state ============ */

/// `FSE_symbolCompressionTransform`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FSE_symbolCompressionTransform {
    pub deltaFindState: i32,
    pub deltaNbBits: U32,
}

/// `FSE_CState_t`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSE_CState_t {
    pub value: isize,
    pub stateTable: *const c_char,
    pub symbolTT: *const c_char,
    pub stateLog: u32,
}

impl Default for FSE_CState_t {
    fn default() -> Self {
        FSE_CState_t {
            value: 0,
            stateTable: core::ptr::null(),
            symbolTT: core::ptr::null(),
            stateLog: 0,
        }
    }
}

/// `FSE_initCState()`
#[inline(always)]
pub unsafe fn fse_init_cstate(state_ptr: &mut FSE_CState_t, ct: *const FSE_CTable) {
    let ptr = ct as *const u8;
    let table_log = mem_read16(ptr) as U32;
    state_ptr.value = 1isize << table_log;
    state_ptr.stateTable = ptr.add(4) as *const c_char; /* u16ptr + 2 */
    state_ptr.symbolTT = ct
        .add(1 + if table_log != 0 { 1 << (table_log - 1) } else { 1 } as usize)
        as *const c_char;
    state_ptr.stateLog = table_log;
}

/// `FSE_initCState2()`
#[inline(always)]
pub unsafe fn fse_init_cstate2(state_ptr: &mut FSE_CState_t, ct: *const FSE_CTable, symbol: U32) {
    fse_init_cstate(state_ptr, ct);
    let symbol_tt = *(state_ptr.symbolTT as *const FSE_symbolCompressionTransform)
        .add(symbol as usize);
    let state_table = state_ptr.stateTable as *const U16;
    let nb_bits_out = (symbol_tt.deltaNbBits.wrapping_add(1 << 15)) >> 16;
    /* C: `(nbBitsOut << 16) - symbolTT.deltaNbBits` is computed in U32 and then
     * widened (value-preserving) into a ptrdiff_t. */
    state_ptr.value = ((nb_bits_out << 16).wrapping_sub(symbol_tt.deltaNbBits)) as isize;
    state_ptr.value = *state_table.offset(
        (state_ptr.value >> nb_bits_out) + symbol_tt.deltaFindState as isize,
    ) as isize;
}

/// `FSE_encodeSymbol()`
#[inline(always)]
pub unsafe fn fse_encode_symbol(
    bit_c: &mut BIT_CStream_t,
    state_ptr: &mut FSE_CState_t,
    symbol: u32,
) {
    let symbol_tt = *(state_ptr.symbolTT as *const FSE_symbolCompressionTransform)
        .add(symbol as usize);
    let state_table = state_ptr.stateTable as *const U16;
    let nb_bits_out =
        ((state_ptr.value.wrapping_add(symbol_tt.deltaNbBits as isize)) >> 16) as u32;
    bit_add_bits(bit_c, state_ptr.value as BitContainerType, nb_bits_out);
    state_ptr.value = *state_table
        .offset((state_ptr.value >> nb_bits_out) + symbol_tt.deltaFindState as isize)
        as isize;
}

/// `FSE_flushCState()`
#[inline(always)]
pub unsafe fn fse_flush_cstate(bit_c: &mut BIT_CStream_t, state_ptr: &FSE_CState_t) {
    bit_add_bits(
        bit_c,
        state_ptr.value as BitContainerType,
        state_ptr.stateLog,
    );
    bit_flush_bits(bit_c);
}

/// `FSE_getMaxNbBits()`
#[inline(always)]
pub unsafe fn fse_get_max_nb_bits(
    symbol_tt_ptr: *const FSE_symbolCompressionTransform,
    symbol_value: U32,
) -> U32 {
    ((*symbol_tt_ptr.add(symbol_value as usize))
        .deltaNbBits
        .wrapping_add((1 << 16) - 1))
        >> 16
}

/// `FSE_bitCost()`
#[inline(always)]
pub unsafe fn fse_bit_cost(
    symbol_tt_ptr: *const FSE_symbolCompressionTransform,
    table_log: U32,
    symbol_value: U32,
    accuracy_log: U32,
) -> U32 {
    let delta_nb_bits = (*symbol_tt_ptr.add(symbol_value as usize)).deltaNbBits;
    let min_nb_bits = delta_nb_bits >> 16;
    let threshold = (min_nb_bits + 1) << 16;
    let table_size = 1u32 << table_log;
    let delta_from_threshold = threshold.wrapping_sub(delta_nb_bits.wrapping_add(table_size));
    let normalized_delta_from_threshold = (delta_from_threshold << accuracy_log) >> table_log;
    let bit_multiplier = 1u32 << accuracy_log;
    (min_nb_bits + 1)
        .wrapping_mul(bit_multiplier)
        .wrapping_sub(normalized_delta_from_threshold)
}

/* ============ decompression state ============ */

/// `FSE_DTableHeader`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FSE_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
}

/// `FSE_decode_t`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FSE_decode_t {
    pub newState: u16,
    pub symbol: u8,
    pub nbBits: u8,
}

/// `FSE_DState_t`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSE_DState_t {
    pub state: usize,
    pub table: *const c_char,
}

impl Default for FSE_DState_t {
    fn default() -> Self {
        FSE_DState_t {
            state: 0,
            table: core::ptr::null(),
        }
    }
}

/// `FSE_initDState()`
#[inline(always)]
pub unsafe fn fse_init_dstate(
    d_state_ptr: &mut FSE_DState_t,
    bit_d: &mut BIT_DStream_t,
    dt: *const FSE_DTable,
) {
    let dtable_h = &*(dt as *const FSE_DTableHeader);
    d_state_ptr.state = bit_read_bits(bit_d, dtable_h.tableLog as u32);
    bit_reload_dstream(bit_d);
    d_state_ptr.table = dt.add(1) as *const c_char;
}

/// `FSE_peekSymbol()`
#[inline(always)]
pub unsafe fn fse_peek_symbol(d_state_ptr: &FSE_DState_t) -> u8 {
    (*(d_state_ptr.table as *const FSE_decode_t).add(d_state_ptr.state)).symbol
}

/// `FSE_updateState()`
#[inline(always)]
pub unsafe fn fse_update_state(d_state_ptr: &mut FSE_DState_t, bit_d: &mut BIT_DStream_t) {
    let d_info = *(d_state_ptr.table as *const FSE_decode_t).add(d_state_ptr.state);
    let nb_bits = d_info.nbBits as u32;
    let low_bits = bit_read_bits(bit_d, nb_bits);
    d_state_ptr.state = d_info.newState as usize + low_bits;
}

/// `FSE_decodeSymbol()`
#[inline(always)]
pub unsafe fn fse_decode_symbol(d_state_ptr: &mut FSE_DState_t, bit_d: &mut BIT_DStream_t) -> u8 {
    let d_info = *(d_state_ptr.table as *const FSE_decode_t).add(d_state_ptr.state);
    let nb_bits = d_info.nbBits as u32;
    let symbol = d_info.symbol;
    let low_bits = bit_read_bits(bit_d, nb_bits);
    d_state_ptr.state = d_info.newState as usize + low_bits;
    symbol
}

/// `FSE_decodeSymbolFast()`
#[inline(always)]
pub unsafe fn fse_decode_symbol_fast(
    d_state_ptr: &mut FSE_DState_t,
    bit_d: &mut BIT_DStream_t,
) -> u8 {
    let d_info = *(d_state_ptr.table as *const FSE_decode_t).add(d_state_ptr.state);
    let nb_bits = d_info.nbBits as u32;
    let symbol = d_info.symbol;
    let low_bits = bit_read_bits_fast(bit_d, nb_bits);
    d_state_ptr.state = d_info.newState as usize + low_bits;
    symbol
}

/// `FSE_endOfDState()`
#[inline(always)]
pub fn fse_end_of_dstate(d_state_ptr: &FSE_DState_t) -> bool {
    d_state_ptr.state == 0
}

/* ============ exported error / version helpers (entropy_common.c) ============ */

#[unsafe(no_mangle)]
pub extern "C" fn FSE_versionNumber() -> c_uint {
    FSE_VERSION_NUMBER
}

#[unsafe(no_mangle)]
pub extern "C" fn FSE_isError(code: usize) -> c_uint {
    err_is_error(code) as c_uint
}

#[unsafe(no_mangle)]
pub extern "C" fn FSE_getErrorName(code: usize) -> *const c_char {
    err_get_error_name(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn HUF_isError(code: usize) -> c_uint {
    err_is_error(code) as c_uint
}

#[unsafe(no_mangle)]
pub extern "C" fn HUF_getErrorName(code: usize) -> *const c_char {
    err_get_error_name(code)
}
