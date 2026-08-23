//! Translation of common/fse.h (types, constants and inlined functions)
#![allow(non_snake_case, dead_code, non_upper_case_globals, non_camel_case_types)]

use crate::bitstream::*;
use crate::mem::*;

pub const FSE_VERSION_MAJOR: u32 = 0;
pub const FSE_VERSION_MINOR: u32 = 9;
pub const FSE_VERSION_RELEASE: u32 = 0;
pub const FSE_VERSION_NUMBER: u32 =
    FSE_VERSION_MAJOR * 100 * 100 + FSE_VERSION_MINOR * 100 + FSE_VERSION_RELEASE;

pub type FSE_CTable = core::ffi::c_uint;
pub type FSE_DTable = core::ffi::c_uint;

pub const FSE_NCOUNTBOUND: usize = 512;
#[inline(always)]
pub fn FSE_BLOCKBOUND(size: usize) -> usize {
    size + (size >> 7) + 4 + core::mem::size_of::<usize>()
}
#[inline(always)]
pub fn FSE_COMPRESSBOUND(size: usize) -> usize {
    FSE_NCOUNTBOUND + FSE_BLOCKBOUND(size)
}

#[inline(always)]
pub const fn FSE_CTABLE_SIZE_U32(maxTableLog: u32, maxSymbolValue: u32) -> usize {
    (1 + (1usize << (maxTableLog - 1)) + ((maxSymbolValue as usize + 1) * 2)) as usize
}
#[inline(always)]
pub const fn FSE_DTABLE_SIZE_U32(maxTableLog: u32) -> usize {
    1 + (1usize << maxTableLog)
}
#[inline(always)]
pub const fn FSE_CTABLE_SIZE(maxTableLog: u32, maxSymbolValue: u32) -> usize {
    FSE_CTABLE_SIZE_U32(maxTableLog, maxSymbolValue) * core::mem::size_of::<FSE_CTable>()
}
#[inline(always)]
pub const fn FSE_DTABLE_SIZE(maxTableLog: u32) -> usize {
    FSE_DTABLE_SIZE_U32(maxTableLog) * core::mem::size_of::<FSE_DTable>()
}

#[inline(always)]
pub const fn FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(maxSymbolValue: u32, tableLog: u32) -> usize {
    (((maxSymbolValue as u64 + 2) + (1u64 << tableLog)) / 2) as usize
        + core::mem::size_of::<U64>() / core::mem::size_of::<U32>()
}
#[inline(always)]
pub const fn FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue: u32, tableLog: u32) -> usize {
    core::mem::size_of::<core::ffi::c_uint>()
        * FSE_BUILD_CTABLE_WORKSPACE_SIZE_U32(maxSymbolValue, tableLog)
}

#[inline(always)]
pub const fn FSE_BUILD_DTABLE_WKSP_SIZE(maxTableLog: u32, maxSymbolValue: u32) -> usize {
    core::mem::size_of::<i16>() * (maxSymbolValue as usize + 1) + (1usize << maxTableLog) + 8
}
#[inline(always)]
pub const fn FSE_BUILD_DTABLE_WKSP_SIZE_U32(maxTableLog: u32, maxSymbolValue: u32) -> usize {
    (FSE_BUILD_DTABLE_WKSP_SIZE(maxTableLog, maxSymbolValue)
        + core::mem::size_of::<core::ffi::c_uint>()
        - 1)
        / core::mem::size_of::<core::ffi::c_uint>()
}
#[inline(always)]
pub const fn FSE_DECOMPRESS_WKSP_SIZE_U32(maxTableLog: u32, maxSymbolValue: u32) -> usize {
    FSE_DTABLE_SIZE_U32(maxTableLog)
        + 1
        + FSE_BUILD_DTABLE_WKSP_SIZE_U32(maxTableLog, maxSymbolValue)
        + (FSE_MAX_SYMBOL_VALUE as usize + 1) / 2
        + 1
}
#[inline(always)]
pub const fn FSE_DECOMPRESS_WKSP_SIZE(maxTableLog: u32, maxSymbolValue: u32) -> usize {
    FSE_DECOMPRESS_WKSP_SIZE_U32(maxTableLog, maxSymbolValue)
        * core::mem::size_of::<core::ffi::c_uint>()
}

pub type FSE_repeat = core::ffi::c_int;
pub const FSE_repeat_none: FSE_repeat = 0;
pub const FSE_repeat_check: FSE_repeat = 1;
pub const FSE_repeat_valid: FSE_repeat = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSE_CState_t {
    pub value: isize,
    pub stateTable: *const core::ffi::c_void,
    pub symbolTT: *const core::ffi::c_void,
    pub stateLog: core::ffi::c_uint,
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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FSE_DState_t {
    pub state: usize,
    pub table: *const core::ffi::c_void,
}

impl Default for FSE_DState_t {
    fn default() -> Self {
        FSE_DState_t {
            state: 0,
            table: core::ptr::null(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FSE_symbolCompressionTransform {
    pub deltaFindState: core::ffi::c_int,
    pub deltaNbBits: U32,
}

#[inline(always)]
pub unsafe fn FSE_initCState(statePtr: *mut FSE_CState_t, ct: *const FSE_CTable) {
    let ptr = ct as *const u8;
    let u16ptr = ptr as *const U16;
    let tableLog: U32 = MEM_read16(ptr) as U32;
    (*statePtr).value = 1isize << tableLog;
    (*statePtr).stateTable = u16ptr.add(2) as *const core::ffi::c_void;
    (*statePtr).symbolTT = ct.add(1 + if tableLog != 0 {
        1usize << (tableLog - 1)
    } else {
        1
    }) as *const core::ffi::c_void;
    (*statePtr).stateLog = tableLog;
}

#[inline(always)]
pub unsafe fn FSE_initCState2(statePtr: *mut FSE_CState_t, ct: *const FSE_CTable, symbol: U32) {
    FSE_initCState(statePtr, ct);
    {
        let symbolTT = *((*statePtr).symbolTT as *const FSE_symbolCompressionTransform)
            .add(symbol as usize);
        let stateTable = (*statePtr).stateTable as *const U16;
        let nbBitsOut: U32 = (symbolTT.deltaNbBits + (1 << 15)) >> 16;
        (*statePtr).value = ((nbBitsOut << 16) as i64 - symbolTT.deltaNbBits as i64) as isize;
        (*statePtr).value = *stateTable.offset(
            ((*statePtr).value >> nbBitsOut) + symbolTT.deltaFindState as isize,
        ) as isize;
    }
}

#[inline(always)]
pub unsafe fn FSE_encodeSymbol(
    bitC: *mut BIT_CStream_t,
    statePtr: *mut FSE_CState_t,
    symbol: core::ffi::c_uint,
) {
    let symbolTT =
        *((*statePtr).symbolTT as *const FSE_symbolCompressionTransform).add(symbol as usize);
    let stateTable = (*statePtr).stateTable as *const U16;
    let nbBitsOut: U32 = (((*statePtr).value as u64).wrapping_add(symbolTT.deltaNbBits as u64)
        >> 16) as U32;
    BIT_addBits(bitC, (*statePtr).value as BitContainerType, nbBitsOut);
    (*statePtr).value =
        *stateTable.offset(((*statePtr).value >> nbBitsOut) + symbolTT.deltaFindState as isize)
            as isize;
}

#[inline(always)]
pub unsafe fn FSE_flushCState(bitC: *mut BIT_CStream_t, statePtr: *const FSE_CState_t) {
    BIT_addBits(
        bitC,
        (*statePtr).value as BitContainerType,
        (*statePtr).stateLog,
    );
    BIT_flushBits(bitC);
}

#[inline(always)]
pub unsafe fn FSE_getMaxNbBits(symbolTTPtr: *const core::ffi::c_void, symbolValue: U32) -> U32 {
    let symbolTT = symbolTTPtr as *const FSE_symbolCompressionTransform;
    ((*symbolTT.add(symbolValue as usize)).deltaNbBits.wrapping_add((1u32 << 16) - 1)) >> 16
}

#[inline(always)]
pub unsafe fn FSE_bitCost(
    symbolTTPtr: *const core::ffi::c_void,
    tableLog: U32,
    symbolValue: U32,
    accuracyLog: U32,
) -> U32 {
    let symbolTT = symbolTTPtr as *const FSE_symbolCompressionTransform;
    let minNbBits: U32 = (*symbolTT.add(symbolValue as usize)).deltaNbBits >> 16;
    let threshold: U32 = (minNbBits + 1) << 16;
    {
        let tableSize: U32 = 1 << tableLog;
        let deltaFromThreshold: U32 = threshold
            .wrapping_sub((*symbolTT.add(symbolValue as usize)).deltaNbBits.wrapping_add(tableSize));
        let normalizedDeltaFromThreshold: U32 = (deltaFromThreshold << accuracyLog) >> tableLog;
        let bitMultiplier: U32 = 1 << accuracyLog;
        (minNbBits + 1)
            .wrapping_mul(bitMultiplier)
            .wrapping_sub(normalizedDeltaFromThreshold)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FSE_DTableHeader {
    pub tableLog: U16,
    pub fastMode: U16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FSE_decode_t {
    pub newState: core::ffi::c_ushort,
    pub symbol: core::ffi::c_uchar,
    pub nbBits: core::ffi::c_uchar,
}

#[inline(always)]
pub unsafe fn FSE_initDState(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
    dt: *const FSE_DTable,
) {
    let ptr = dt as *const u8;
    let DTableH = ptr as *const FSE_DTableHeader;
    (*DStatePtr).state = BIT_readBits(bitD, (*DTableH).tableLog as u32);
    BIT_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1) as *const core::ffi::c_void;
}

#[inline(always)]
pub unsafe fn FSE_peekSymbol(DStatePtr: *const FSE_DState_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    DInfo.symbol
}

#[inline(always)]
pub unsafe fn FSE_updateState(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let lowBits = BIT_readBits(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
}

#[inline(always)]
pub unsafe fn FSE_decodeSymbol(DStatePtr: *mut FSE_DState_t, bitD: *mut BIT_DStream_t) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BIT_readBits(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline(always)]
pub unsafe fn FSE_decodeSymbolFast(
    DStatePtr: *mut FSE_DState_t,
    bitD: *mut BIT_DStream_t,
) -> BYTE {
    let DInfo = *((*DStatePtr).table as *const FSE_decode_t).add((*DStatePtr).state);
    let nbBits: U32 = DInfo.nbBits as U32;
    let symbol: BYTE = DInfo.symbol;
    let lowBits = BIT_readBitsFast(bitD, nbBits);
    (*DStatePtr).state = DInfo.newState as usize + lowBits;
    symbol
}

#[inline(always)]
pub unsafe fn FSE_endOfDState(DStatePtr: *const FSE_DState_t) -> core::ffi::c_uint {
    ((*DStatePtr).state == 0) as core::ffi::c_uint
}

pub const FSE_MAX_MEMORY_USAGE: u32 = 14;
pub const FSE_DEFAULT_MEMORY_USAGE: u32 = 13;
pub const FSE_MAX_SYMBOL_VALUE: u32 = 255;

pub const FSE_MAX_TABLELOG: u32 = FSE_MAX_MEMORY_USAGE - 2;
pub const FSE_MAX_TABLESIZE: u32 = 1 << FSE_MAX_TABLELOG;
pub const FSE_MAXTABLESIZE_MASK: u32 = FSE_MAX_TABLESIZE - 1;
pub const FSE_DEFAULT_TABLELOG: u32 = FSE_DEFAULT_MEMORY_USAGE - 2;
pub const FSE_MIN_TABLELOG: u32 = 5;
pub const FSE_TABLELOG_ABSOLUTE_MAX: u32 = 15;

#[inline(always)]
pub fn FSE_TABLESTEP(tableSize: u32) -> u32 {
    (tableSize >> 1) + (tableSize >> 3) + 3
}
