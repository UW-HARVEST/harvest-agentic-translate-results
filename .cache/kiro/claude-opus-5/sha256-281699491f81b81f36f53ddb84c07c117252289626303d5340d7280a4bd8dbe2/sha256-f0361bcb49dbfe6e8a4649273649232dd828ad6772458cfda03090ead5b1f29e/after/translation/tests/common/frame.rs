//! Shared `lz4frame` FFI declarations and preference-matrix helpers.
#![allow(dead_code)]

/// `LZ4F_frameInfo_t` — field order and types straight from `lz4frame.h:174`.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct FrameInfo {
    pub block_size_id: i32,
    pub block_mode: i32,
    pub content_checksum_flag: i32,
    pub frame_type: i32,
    pub content_size: u64,
    pub dict_id: u32,
    pub block_checksum_flag: i32,
}

/// `LZ4F_preferences_t` — `lz4frame.h:191`.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Prefs {
    pub frame_info: FrameInfo,
    pub compression_level: i32,
    pub auto_flush: u32,
    pub favor_dec_speed: u32,
    pub reserved: [u32; 3],
}

/// `LZ4F_compressOptions_t` — `lz4frame.h:250`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct CompressOptions {
    pub stable_src: u32,
    pub reserved: [u32; 3],
}

/// `LZ4F_decompressOptions_t` — `lz4frame.h:371`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct DecompressOptions {
    pub stable_dst: u32,
    pub skip_checksums: u32,
    pub reserved1: u32,
    pub reserved0: u32,
}

pub const LZ4F_VERSION: u32 = 100;
pub const HEADER_SIZE_MAX: usize = 19;

// ------------------------------------------------------------ signatures -----

pub type FnCreateCctx = unsafe extern "C" fn(*mut *mut u8, u32) -> usize;
pub type FnFreeCctx = unsafe extern "C" fn(*mut u8) -> usize;
pub type FnCreateDctx = unsafe extern "C" fn(*mut *mut u8, u32) -> usize;
pub type FnFreeDctx = unsafe extern "C" fn(*mut u8) -> usize;
pub type FnResetDctx = unsafe extern "C" fn(*mut u8);
pub type FnCompressBegin = unsafe extern "C" fn(*mut u8, *mut u8, usize, *const Prefs) -> usize;
pub type FnCompressBeginDict =
    unsafe extern "C" fn(*mut u8, *mut u8, usize, *const u8, usize, *const Prefs) -> usize;
pub type FnCompressBeginCDict =
    unsafe extern "C" fn(*mut u8, *mut u8, usize, *const u8, *const Prefs) -> usize;
pub type FnFrameBound = unsafe extern "C" fn(usize, *const Prefs) -> usize;
pub type FnCompressUpdate = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const CompressOptions,
) -> usize;
pub type FnFlush = unsafe extern "C" fn(*mut u8, *mut u8, usize, *const CompressOptions) -> usize;
pub type FnCompressFrame =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const Prefs) -> usize;
/// `LZ4F_compressFrame_usingCDict(LZ4F_cctx*, dst, dstCapacity, src, srcSize,
/// cdict, prefs)` — note the leading cctx (lz4frame.h:599).
pub type FnCompressFrameCDict = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *const Prefs,
) -> usize;
pub type FnCreateCDict = unsafe extern "C" fn(*const u8, usize) -> *mut u8;

/// `LZ4F_CustomMem` — four pointers, passed BY VALUE (lz4frame.h:729-735).
/// All-NULL means "defer to stdlib", i.e. `LZ4F_defaultCMem`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct CustomMem {
    pub custom_alloc: *const (),
    pub custom_calloc: *const (),
    pub custom_free: *const (),
    pub opaque_state: *mut (),
}
impl CustomMem {
    pub fn default_cmem() -> Self {
        CustomMem {
            custom_alloc: std::ptr::null(),
            custom_calloc: std::ptr::null(),
            custom_free: std::ptr::null(),
            opaque_state: std::ptr::null_mut(),
        }
    }
}
pub type FnCreateCDictAdvanced =
    unsafe extern "C" fn(CustomMem, *const u8, usize) -> *mut u8;
pub type FnCreateCctxAdvanced = unsafe extern "C" fn(CustomMem, u32) -> *mut u8;
pub type FnCreateDctxAdvanced = unsafe extern "C" fn(CustomMem, u32) -> *mut u8;
pub type FnFreeCDict = unsafe extern "C" fn(*mut u8);
pub type FnHeaderSize = unsafe extern "C" fn(*const u8, usize) -> usize;
pub type FnGetFrameInfo =
    unsafe extern "C" fn(*mut u8, *mut FrameInfo, *const u8, *mut usize) -> usize;
pub type FnDecompress = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut usize,
    *const u8,
    *mut usize,
    *const DecompressOptions,
) -> usize;
pub type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *mut usize,
    *const u8,
    *mut usize,
    *const u8,
    usize,
    *const DecompressOptions,
) -> usize;
pub type FnGetBlockSize = unsafe extern "C" fn(u32) -> usize;
pub type FnIsError = unsafe extern "C" fn(usize) -> u32;
pub type FnGetErrorName = unsafe extern "C" fn(usize) -> *const std::os::raw::c_char;
pub type FnGetErrorCode = unsafe extern "C" fn(usize) -> i32;
pub type FnGetVersion = unsafe extern "C" fn() -> u32;
pub type FnLevelMax = unsafe extern "C" fn() -> i32;

// -------------------------------------------------------- prefs matrices -----

/// The block-size IDs the C accepts (`0` means default = 64 KB).
pub const VALID_BSID: [i32; 5] = [0, 4, 5, 6, 7];
/// Compression levels spanning fast-acceleration, fast, HC and optimal, plus clamps.
pub const LEVELS: [i32; 12] = [-65537, -5, -1, 0, 1, 2, 3, 6, 9, 10, 12, 100];

/// The full cross-product of preference axes the C branches on, pruned to the
/// combinations that produce distinct behaviour (rows 118-136 of CONFIGS.md).
pub fn prefs_matrix() -> Vec<(String, Prefs)> {
    let mut v = Vec::new();
    for &bsid in VALID_BSID.iter() {
        for &bmode in &[0i32, 1] {
            for &cck in &[0i32, 1] {
                for &bck in &[0i32, 1] {
                    for &lvl in &[0i32, 9] {
                        for &af in &[0u32, 1] {
                            v.push((
                                format!(
                                    "bsid={bsid} bmode={bmode} cck={cck} bck={bck} lvl={lvl} af={af}"
                                ),
                                Prefs {
                                    frame_info: FrameInfo {
                                        block_size_id: bsid,
                                        block_mode: bmode,
                                        content_checksum_flag: cck,
                                        block_checksum_flag: bck,
                                        ..Default::default()
                                    },
                                    compression_level: lvl,
                                    auto_flush: af,
                                    ..Default::default()
                                },
                            ));
                        }
                    }
                }
            }
        }
    }
    // level sweep (rows 128-133) and favorDecSpeed (row 135) at one shape
    for &lvl in LEVELS.iter() {
        for &fav in &[0u32, 1] {
            v.push((
                format!("lvl={lvl} favorDecSpeed={fav}"),
                Prefs {
                    frame_info: FrameInfo {
                        block_size_id: 4,
                        content_checksum_flag: 1,
                        block_checksum_flag: 1,
                        ..Default::default()
                    },
                    compression_level: lvl,
                    favor_dec_speed: fav,
                    ..Default::default()
                },
            ));
        }
    }
    // dictID (row 127) and skippable frameType (row 136)
    for &(dict_id, ftype) in &[(0u32, 0i32), (0xDEAD_BEEFu32, 0i32), (0u32, 1i32)] {
        v.push((
            format!("dictID={dict_id:#x} frameType={ftype}"),
            Prefs {
                frame_info: FrameInfo {
                    block_size_id: 5,
                    dict_id,
                    frame_type: ftype,
                    ..Default::default()
                },
                ..Default::default()
            },
        ));
    }
    v
}

/// A smaller matrix for the expensive low-level pipeline tests.
pub fn prefs_matrix_small() -> Vec<(String, Prefs)> {
    let mut v = Vec::new();
    for &bsid in &[0i32, 4, 5, 7] {
        for &bmode in &[0i32, 1] {
            for &cck in &[0i32, 1] {
                for &bck in &[0i32, 1] {
                    for &lvl in &[0i32, -3, 3, 12] {
                        for &af in &[0u32, 1] {
                            v.push((
                                format!(
                                    "bsid={bsid} bmode={bmode} cck={cck} bck={bck} lvl={lvl} af={af}"
                                ),
                                Prefs {
                                    frame_info: FrameInfo {
                                        block_size_id: bsid,
                                        block_mode: bmode,
                                        content_checksum_flag: cck,
                                        block_checksum_flag: bck,
                                        ..Default::default()
                                    },
                                    compression_level: lvl,
                                    auto_flush: af,
                                    ..Default::default()
                                },
                            ));
                        }
                    }
                }
            }
        }
    }
    v
}
