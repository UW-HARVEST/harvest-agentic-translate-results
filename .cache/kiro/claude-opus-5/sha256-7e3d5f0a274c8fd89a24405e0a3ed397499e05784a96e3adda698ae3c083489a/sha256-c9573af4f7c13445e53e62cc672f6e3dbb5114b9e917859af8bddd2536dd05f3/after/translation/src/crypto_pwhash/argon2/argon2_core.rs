//! Translation of `crypto_pwhash/argon2/argon2-core.c`
//! plus the shared C types/constants from `argon2.h` and `argon2-core.h`.
//!
//! Build facts: no `HAVE_*` macros are defined, so the portable memory
//! allocator path is used (`malloc(memory_size + 63)` with 64-byte manual
//! alignment) and `argon2_pick_best_implementation()` always selects
//! `argon2_fill_segment_ref`.

use core::ffi::{c_int, c_void};

use crate::common::{load64_le, store32_le, store64_le};
use crate::crypto_generichash::blake2b::{
    crypto_generichash_blake2b_final, crypto_generichash_blake2b_init,
    crypto_generichash_blake2b_state, crypto_generichash_blake2b_update,
};
use crate::sodium_utils::sodium_memzero;

use super::blake2b_long::_sodium_blake2b_long;
use super::fill_block_ref::_sodium_argon2_fill_segment_ref;

/* ===================================================================
 * argon2.h — input parameter restrictions
 * =================================================================== */

pub const ARGON2_MIN_LANES: u32 = 1;
pub const ARGON2_MAX_LANES: u32 = 0xFFFFFF;

pub const ARGON2_MIN_THREADS: u32 = 1;
pub const ARGON2_MAX_THREADS: u32 = 0xFFFFFF;

pub const ARGON2_SYNC_POINTS: u32 = 4;

pub const ARGON2_MIN_OUTLEN: u32 = 16;
pub const ARGON2_MAX_OUTLEN: u32 = 0xFFFFFFFF;

pub const ARGON2_MIN_MEMORY: u32 = 2 * ARGON2_SYNC_POINTS; /* 2 blocks per slice */

/* Max memory size is half the addressing space, topping at 2^32 blocks.
 * On LP64: sizeof(void*)*CHAR_BIT - 10 - 1 = 64 - 11 = 53, min(32, 53) = 32.
 * ARGON2_MAX_MEMORY = min(0xFFFFFFFF, 1<<32) = 0xFFFFFFFF. */
pub const ARGON2_MAX_MEMORY_BITS: u64 = 32;
pub const ARGON2_MAX_MEMORY: u64 = 0xFFFFFFFF;

pub const ARGON2_MIN_TIME: u32 = 1;
pub const ARGON2_MAX_TIME: u32 = 0xFFFFFFFF;

pub const ARGON2_MIN_PWD_LENGTH: u32 = 0;
pub const ARGON2_MAX_PWD_LENGTH: u32 = 0xFFFFFFFF;

pub const ARGON2_MIN_AD_LENGTH: u32 = 0;
pub const ARGON2_MAX_AD_LENGTH: u32 = 0xFFFFFFFF;

pub const ARGON2_MIN_SALT_LENGTH: u32 = 8;
pub const ARGON2_MAX_SALT_LENGTH: u32 = 0xFFFFFFFF;

pub const ARGON2_MIN_SECRET: u32 = 0;
pub const ARGON2_MAX_SECRET: u32 = 0xFFFFFFFF;

pub const ARGON2_FLAG_CLEAR_PASSWORD: u32 = 1 << 0;
pub const ARGON2_FLAG_CLEAR_SECRET: u32 = 1 << 1;
pub const ARGON2_DEFAULT_FLAGS: u32 = 0;

/* ---- error codes (typedef enum Argon2_ErrorCodes / argon2_error_codes) ---- */
pub type argon2_error_codes = c_int;

pub const ARGON2_OK: c_int = 0;
pub const ARGON2_OUTPUT_PTR_NULL: c_int = -1;
pub const ARGON2_OUTPUT_TOO_SHORT: c_int = -2;
pub const ARGON2_OUTPUT_TOO_LONG: c_int = -3;
pub const ARGON2_PWD_TOO_SHORT: c_int = -4;
pub const ARGON2_PWD_TOO_LONG: c_int = -5;
pub const ARGON2_SALT_TOO_SHORT: c_int = -6;
pub const ARGON2_SALT_TOO_LONG: c_int = -7;
pub const ARGON2_AD_TOO_SHORT: c_int = -8;
pub const ARGON2_AD_TOO_LONG: c_int = -9;
pub const ARGON2_SECRET_TOO_SHORT: c_int = -10;
pub const ARGON2_SECRET_TOO_LONG: c_int = -11;
pub const ARGON2_TIME_TOO_SMALL: c_int = -12;
pub const ARGON2_TIME_TOO_LARGE: c_int = -13;
pub const ARGON2_MEMORY_TOO_LITTLE: c_int = -14;
pub const ARGON2_MEMORY_TOO_MUCH: c_int = -15;
pub const ARGON2_LANES_TOO_FEW: c_int = -16;
pub const ARGON2_LANES_TOO_MANY: c_int = -17;
pub const ARGON2_PWD_PTR_MISMATCH: c_int = -18;
pub const ARGON2_SALT_PTR_MISMATCH: c_int = -19;
pub const ARGON2_SECRET_PTR_MISMATCH: c_int = -20;
pub const ARGON2_AD_PTR_MISMATCH: c_int = -21;
pub const ARGON2_MEMORY_ALLOCATION_ERROR: c_int = -22;
pub const ARGON2_FREE_MEMORY_CBK_NULL: c_int = -23;
pub const ARGON2_ALLOCATE_MEMORY_CBK_NULL: c_int = -24;
pub const ARGON2_INCORRECT_PARAMETER: c_int = -25;
pub const ARGON2_INCORRECT_TYPE: c_int = -26;
pub const ARGON2_OUT_PTR_MISMATCH: c_int = -27;
pub const ARGON2_THREADS_TOO_FEW: c_int = -28;
pub const ARGON2_THREADS_TOO_MANY: c_int = -29;
pub const ARGON2_MISSING_ARGS: c_int = -30;
pub const ARGON2_ENCODING_FAIL: c_int = -31;
pub const ARGON2_DECODING_FAIL: c_int = -32;
pub const ARGON2_THREAD_FAIL: c_int = -33;
pub const ARGON2_DECODING_LENGTH_FAIL: c_int = -34;
pub const ARGON2_VERIFY_MISMATCH: c_int = -35;

/* ---- typedef struct Argon2_Context (argon2_context) ---- */
#[repr(C)]
pub struct argon2_context {
    pub out: *mut u8,
    pub outlen: u32,

    pub pwd: *mut u8,
    pub pwdlen: u32,

    pub salt: *mut u8,
    pub saltlen: u32,

    pub secret: *mut u8,
    pub secretlen: u32,

    pub ad: *mut u8,
    pub adlen: u32,

    pub t_cost: u32,
    pub m_cost: u32,
    pub lanes: u32,
    pub threads: u32,

    pub flags: u32,
}

/* ---- typedef enum Argon2_type (argon2_type) ---- */
pub type argon2_type = c_int;
pub const Argon2_i: c_int = 1;
pub const Argon2_id: c_int = 2;

/* ===================================================================
 * argon2-core.h — internal constants
 * =================================================================== */

pub const ARGON2_VERSION_NUMBER: u32 = 0x13;

pub const ARGON2_BLOCK_SIZE: usize = 1024;
pub const ARGON2_QWORDS_IN_BLOCK: usize = ARGON2_BLOCK_SIZE / 8;
pub const ARGON2_OWORDS_IN_BLOCK: usize = ARGON2_BLOCK_SIZE / 16;
pub const ARGON2_HWORDS_IN_BLOCK: usize = ARGON2_BLOCK_SIZE / 32;
pub const ARGON2_512BIT_WORDS_IN_BLOCK: usize = ARGON2_BLOCK_SIZE / 64;

pub const ARGON2_ADDRESSES_IN_BLOCK: usize = 128;

pub const ARGON2_PREHASH_DIGEST_LENGTH: usize = 64;
pub const ARGON2_PREHASH_SEED_LENGTH: usize = 72;

/* ---- typedef struct block_ (block) ---- */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct block {
    pub v: [u64; ARGON2_QWORDS_IN_BLOCK],
}

/* ---- typedef struct block_region_ (block_region) ---- */
#[repr(C)]
pub struct block_region {
    pub base: *mut c_void,
    pub memory: *mut block,
    pub size: usize,
}

/* ---- typedef struct Argon2_instance_t (argon2_instance_t) ---- */
#[repr(C)]
pub struct argon2_instance_t {
    pub region: *mut block_region,
    pub pseudo_rands: *mut u64,
    pub passes: u32,
    pub current_pass: u32,
    pub memory_blocks: u32,
    pub segment_length: u32,
    pub lane_length: u32,
    pub lanes: u32,
    pub threads: u32,
    pub type_: argon2_type,
    pub print_internals: c_int,
}

/* ---- typedef struct Argon2_position_t (argon2_position_t) ---- */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct argon2_position_t {
    pub pass: u32,
    pub lane: u32,
    pub slice: u8,
    pub index: u32,
}

/* ---- typedef struct Argon2_thread_data (argon2_thread_data) ---- */
#[repr(C)]
pub struct argon2_thread_data {
    pub instance_ptr: *mut argon2_instance_t,
    pub pos: argon2_position_t,
}

/* ---- fill_segment_fn ---- */
pub type fill_segment_fn =
    unsafe extern "C" fn(instance: *const argon2_instance_t, position: argon2_position_t);

/* =====================================================================
 * Functions that work with the block (static inline in argon2-core.h)
 * ===================================================================== */

/* Initialize each byte of the block with @in */
#[inline]
pub unsafe fn init_block_value(b: *mut block, in_: u8) {
    crate::common::memset(
        (*b).v.as_mut_ptr() as *mut u8,
        in_,
        core::mem::size_of_val(&(*b).v),
    );
}

/* Copy block @src to block @dst */
#[inline]
pub unsafe fn copy_block(dst: *mut block, src: *const block) {
    crate::common::memcpy(
        (*dst).v.as_mut_ptr() as *mut u8,
        (*src).v.as_ptr() as *const u8,
        core::mem::size_of::<u64>() * ARGON2_QWORDS_IN_BLOCK,
    );
}

/* XOR @src onto @dst bytewise */
#[inline]
pub unsafe fn xor_block(dst: *mut block, src: *const block) {
    let mut i: i32 = 0;
    while (i as usize) < ARGON2_QWORDS_IN_BLOCK {
        (*dst).v[i as usize] ^= (*src).v[i as usize];
        i += 1;
    }
}

/* =====================================================================
 * index_alpha (static in argon2-core.h)
 * ===================================================================== */
pub unsafe fn index_alpha(
    instance: *const argon2_instance_t,
    position: *const argon2_position_t,
    pseudo_rand: u32,
    same_lane: c_int,
) -> u32 {
    let reference_area_size: u32;
    let mut relative_position: u64;
    let mut absolute_position: u64;
    let start_position: u32;

    if (*position).pass == 0 {
        /* First pass */
        if (*position).slice == 0 {
            /* First slice */
            reference_area_size = (*position).index.wrapping_sub(1); /* all but the previous */
        } else {
            if same_lane != 0 {
                /* The same lane => add current segment */
                reference_area_size = ((*position).slice as u32)
                    .wrapping_mul((*instance).segment_length)
                    .wrapping_add((*position).index)
                    .wrapping_sub(1);
            } else {
                reference_area_size = ((*position).slice as u32)
                    .wrapping_mul((*instance).segment_length)
                    .wrapping_add(if (*position).index == 0 {
                        (-1i32) as u32
                    } else {
                        0
                    });
            }
        }
    } else {
        /* Second pass */
        if same_lane != 0 {
            reference_area_size = (*instance)
                .lane_length
                .wrapping_sub((*instance).segment_length)
                .wrapping_add((*position).index)
                .wrapping_sub(1);
        } else {
            reference_area_size = (*instance)
                .lane_length
                .wrapping_sub((*instance).segment_length)
                .wrapping_add(if (*position).index == 0 {
                    (-1i32) as u32
                } else {
                    0
                });
        }
    }

    /* 1.2.4. Mapping pseudo_rand to 0..<reference_area_size-1> */
    relative_position = pseudo_rand as u64;
    relative_position = (relative_position.wrapping_mul(relative_position)) >> 32;
    relative_position = (reference_area_size as u64)
        .wrapping_sub(1)
        .wrapping_sub((reference_area_size as u64).wrapping_mul(relative_position) >> 32);

    /* 1.2.5 Computing starting position */
    start_position = if (*position).pass != 0 {
        if (*position).slice as u32 == ARGON2_SYNC_POINTS - 1 {
            0
        } else {
            ((*position).slice as u32 + 1).wrapping_mul((*instance).segment_length)
        }
    } else {
        0
    };

    /* 1.2.6. Computing absolute position */
    absolute_position = (start_position as u64)
        .wrapping_add(relative_position)
        .wrapping_sub((*instance).lane_length as u64);
    absolute_position = absolute_position
        .wrapping_add(((*instance).lane_length as u64) & (absolute_position >> 32));
    absolute_position as u32
}

/* =====================================================================
 * argon2-core.c
 * ===================================================================== */

/* static fill_segment_fn fill_segment = argon2_fill_segment_ref; */
struct FillSegmentCell(core::cell::UnsafeCell<fill_segment_fn>);
unsafe impl Sync for FillSegmentCell {}

static fill_segment: FillSegmentCell =
    FillSegmentCell(core::cell::UnsafeCell::new(_sodium_argon2_fill_segment_ref));

#[inline(always)]
unsafe fn call_fill_segment(instance: *const argon2_instance_t, position: argon2_position_t) {
    (*fill_segment.0.get())(instance, position)
}

unsafe fn load_block(dst: *mut block, input: *const c_void) {
    let mut i: u32 = 0;
    while (i as usize) < ARGON2_QWORDS_IN_BLOCK {
        (*dst).v[i as usize] =
            load64_le((input as *const u8).add((i as usize) * core::mem::size_of::<u64>()));
        i += 1;
    }
}

unsafe fn store_block(output: *mut c_void, src: *const block) {
    let mut i: u32 = 0;
    while (i as usize) < ARGON2_QWORDS_IN_BLOCK {
        store64_le(
            (output as *mut u8).add((i as usize) * core::mem::size_of::<u64>()),
            (*src).v[i as usize],
        );
        i += 1;
    }
}

/***************Memory allocators*****************/
unsafe fn allocate_memory(region: *mut *mut block_region, m_cost: u32) -> c_int {
    let base: *mut c_void;
    let mut memory: *mut block;
    let memory_size: usize;

    if region.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    memory_size = core::mem::size_of::<block>().wrapping_mul(m_cost as usize);
    if m_cost == 0 || memory_size / (m_cost as usize) != core::mem::size_of::<block>() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    *region = libc::malloc(core::mem::size_of::<block_region>()) as *mut block_region;
    if (*region).is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    (*(*region)).base = core::ptr::null_mut();
    (*(*region)).memory = core::ptr::null_mut();

    /* Portable path: neither MAP_ANON+HAVE_MMAP nor HAVE_POSIX_MEMALIGN. */
    memory = core::ptr::null_mut();
    if memory_size.wrapping_add(63) < memory_size {
        base = core::ptr::null_mut();
        crate::set_errno(crate::ENOMEM);
    } else {
        base = libc::malloc(memory_size.wrapping_add(63));
        if !base.is_null() {
            let mut aligned: *mut u8 = (base as *mut u8).add(63);
            aligned = aligned.offset(-(((aligned as usize) & 63) as isize));
            memory = aligned as *mut block;
        }
    }
    if base.is_null() {
        libc::free(*region as *mut c_void);
        *region = core::ptr::null_mut();
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    (*(*region)).base = base;
    (*(*region)).memory = memory;
    (*(*region)).size = memory_size;

    ARGON2_OK
}

/*********Memory functions*/
unsafe fn free_memory(region: *mut block_region) {
    if !region.is_null() && !(*region).base.is_null() {
        libc::free((*region).base);
    }
    libc::free(region as *mut c_void);
}

unsafe fn argon2_free_instance(instance: *mut argon2_instance_t, _flags: c_int) {
    libc::free((*instance).pseudo_rands as *mut c_void);
    (*instance).pseudo_rands = core::ptr::null_mut();
    free_memory((*instance).region);
    (*instance).region = core::ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_finalize(
    context: *const argon2_context,
    instance: *mut argon2_instance_t,
) {
    if !context.is_null() && !instance.is_null() {
        let mut blockhash: block = core::mem::zeroed();
        let mut l: u32;

        copy_block(
            &mut blockhash,
            (*(*instance).region)
                .memory
                .add(((*instance).lane_length - 1) as usize),
        );

        /* XOR the last blocks */
        l = 1;
        while l < (*instance).lanes {
            let last_block_in_lane: u32 =
                l * (*instance).lane_length + ((*instance).lane_length - 1);
            xor_block(
                &mut blockhash,
                (*(*instance).region)
                    .memory
                    .add(last_block_in_lane as usize),
            );
            l += 1;
        }

        /* Hash the result */
        {
            let mut blockhash_bytes: [u8; ARGON2_BLOCK_SIZE] = [0; ARGON2_BLOCK_SIZE];
            store_block(blockhash_bytes.as_mut_ptr() as *mut c_void, &blockhash);
            _sodium_blake2b_long(
                (*context).out as *mut c_void,
                (*context).outlen as usize,
                blockhash_bytes.as_ptr() as *const c_void,
                ARGON2_BLOCK_SIZE,
            );
            sodium_memzero(blockhash.v.as_mut_ptr() as *mut c_void, ARGON2_BLOCK_SIZE);
            sodium_memzero(
                blockhash_bytes.as_mut_ptr() as *mut c_void,
                ARGON2_BLOCK_SIZE,
            );
        }

        argon2_free_instance(instance, (*context).flags as c_int);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_fill_memory_blocks(
    instance: *mut argon2_instance_t,
    pass: u32,
) {
    let mut position: argon2_position_t = core::mem::zeroed();
    let mut l: u32;
    let mut s: u32;

    if instance.is_null() || (*instance).lanes == 0 {
        return;
    }

    position.pass = pass;
    s = 0;
    while s < ARGON2_SYNC_POINTS {
        position.slice = s as u8;
        l = 0;
        while l < (*instance).lanes {
            position.lane = l;
            position.index = 0;
            call_fill_segment(instance, position);
            l += 1;
        }
        s += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_validate_inputs(context: *const argon2_context) -> c_int {
    if context.is_null() {
        return ARGON2_INCORRECT_PARAMETER;
    }

    if (*context).out.is_null() {
        return ARGON2_OUTPUT_PTR_NULL;
    }

    /* Validate output length */
    if ARGON2_MIN_OUTLEN > (*context).outlen {
        return ARGON2_OUTPUT_TOO_SHORT;
    }
    if ARGON2_MAX_OUTLEN < (*context).outlen {
        return ARGON2_OUTPUT_TOO_LONG;
    }

    /* Validate password (required param) */
    if (*context).pwd.is_null() {
        if 0 != (*context).pwdlen {
            return ARGON2_PWD_PTR_MISMATCH;
        }
    }
    if ARGON2_MIN_PWD_LENGTH > (*context).pwdlen {
        return ARGON2_PWD_TOO_SHORT;
    }
    if ARGON2_MAX_PWD_LENGTH < (*context).pwdlen {
        return ARGON2_PWD_TOO_LONG;
    }

    /* Validate salt (required param) */
    if (*context).salt.is_null() {
        if 0 != (*context).saltlen {
            return ARGON2_SALT_PTR_MISMATCH;
        }
    }
    if ARGON2_MIN_SALT_LENGTH > (*context).saltlen {
        return ARGON2_SALT_TOO_SHORT;
    }
    if ARGON2_MAX_SALT_LENGTH < (*context).saltlen {
        return ARGON2_SALT_TOO_LONG;
    }

    /* Validate secret (optional param) */
    if (*context).secret.is_null() {
        if 0 != (*context).secretlen {
            return ARGON2_SECRET_PTR_MISMATCH;
        }
    } else {
        if ARGON2_MIN_SECRET > (*context).secretlen {
            return ARGON2_SECRET_TOO_SHORT;
        }
        if ARGON2_MAX_SECRET < (*context).secretlen {
            return ARGON2_SECRET_TOO_LONG;
        }
    }

    /* Validate associated data (optional param) */
    if (*context).ad.is_null() {
        if 0 != (*context).adlen {
            return ARGON2_AD_PTR_MISMATCH;
        }
    } else {
        if ARGON2_MIN_AD_LENGTH > (*context).adlen {
            return ARGON2_AD_TOO_SHORT;
        }
        if ARGON2_MAX_AD_LENGTH < (*context).adlen {
            return ARGON2_AD_TOO_LONG;
        }
    }

    /* Validate lanes */
    if ARGON2_MIN_LANES > (*context).lanes {
        return ARGON2_LANES_TOO_FEW;
    }
    if ARGON2_MAX_LANES < (*context).lanes {
        return ARGON2_LANES_TOO_MANY;
    }

    /* Validate memory cost */
    if ARGON2_MIN_MEMORY > (*context).m_cost {
        return ARGON2_MEMORY_TOO_LITTLE;
    }
    if ARGON2_MAX_MEMORY < (*context).m_cost as u64 {
        return ARGON2_MEMORY_TOO_MUCH;
    }
    if (*context).m_cost < 8u32.wrapping_mul((*context).lanes) {
        return ARGON2_MEMORY_TOO_LITTLE;
    }

    /* Validate time cost */
    if ARGON2_MIN_TIME > (*context).t_cost {
        return ARGON2_TIME_TOO_SMALL;
    }
    if ARGON2_MAX_TIME < (*context).t_cost {
        return ARGON2_TIME_TOO_LARGE;
    }

    /* Validate threads */
    if ARGON2_MIN_THREADS > (*context).threads {
        return ARGON2_THREADS_TOO_FEW;
    }
    if ARGON2_MAX_THREADS < (*context).threads {
        return ARGON2_THREADS_TOO_MANY;
    }

    ARGON2_OK
}

unsafe fn argon2_fill_first_blocks(blockhash: *mut u8, instance: *const argon2_instance_t) {
    let mut l: u32;
    let mut blockhash_bytes: [u8; ARGON2_BLOCK_SIZE] = [0; ARGON2_BLOCK_SIZE];
    l = 0;
    while l < (*instance).lanes {
        store32_le(blockhash.add(ARGON2_PREHASH_DIGEST_LENGTH), 0);
        store32_le(blockhash.add(ARGON2_PREHASH_DIGEST_LENGTH + 4), l);
        _sodium_blake2b_long(
            blockhash_bytes.as_mut_ptr() as *mut c_void,
            ARGON2_BLOCK_SIZE,
            blockhash as *const c_void,
            ARGON2_PREHASH_SEED_LENGTH,
        );
        load_block(
            (*(*instance).region)
                .memory
                .add((l * (*instance).lane_length + 0) as usize),
            blockhash_bytes.as_ptr() as *const c_void,
        );

        store32_le(blockhash.add(ARGON2_PREHASH_DIGEST_LENGTH), 1);
        _sodium_blake2b_long(
            blockhash_bytes.as_mut_ptr() as *mut c_void,
            ARGON2_BLOCK_SIZE,
            blockhash as *const c_void,
            ARGON2_PREHASH_SEED_LENGTH,
        );
        load_block(
            (*(*instance).region)
                .memory
                .add((l * (*instance).lane_length + 1) as usize),
            blockhash_bytes.as_ptr() as *const c_void,
        );
        l += 1;
    }
    sodium_memzero(
        blockhash_bytes.as_mut_ptr() as *mut c_void,
        ARGON2_BLOCK_SIZE,
    );
}

unsafe fn argon2_initial_hash(
    blockhash: *mut u8,
    context: *mut argon2_context,
    type_: argon2_type,
) {
    let mut blake_hash: crypto_generichash_blake2b_state = core::mem::zeroed();
    let mut value: [u8; 4] = [0; 4];

    if context.is_null() || blockhash.is_null() {
        return;
    }

    crypto_generichash_blake2b_init(
        &mut blake_hash,
        core::ptr::null(),
        0,
        ARGON2_PREHASH_DIGEST_LENGTH,
    );

    store32_le(value.as_mut_ptr(), (*context).lanes);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), (*context).outlen);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), (*context).m_cost);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), (*context).t_cost);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), ARGON2_VERSION_NUMBER);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), type_ as u32);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    store32_le(value.as_mut_ptr(), (*context).pwdlen);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    if !(*context).pwd.is_null() {
        crypto_generichash_blake2b_update(
            &mut blake_hash,
            (*context).pwd as *const u8,
            (*context).pwdlen as u64,
        );

        if (*context).flags & ARGON2_FLAG_CLEAR_PASSWORD != 0 {
            sodium_memzero((*context).pwd as *mut c_void, (*context).pwdlen as usize);
            (*context).pwdlen = 0;
        }
    }

    store32_le(value.as_mut_ptr(), (*context).saltlen);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    if !(*context).salt.is_null() {
        crypto_generichash_blake2b_update(
            &mut blake_hash,
            (*context).salt as *const u8,
            (*context).saltlen as u64,
        );
    }

    store32_le(value.as_mut_ptr(), (*context).secretlen);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    if !(*context).secret.is_null() {
        crypto_generichash_blake2b_update(
            &mut blake_hash,
            (*context).secret as *const u8,
            (*context).secretlen as u64,
        );
        if (*context).flags & ARGON2_FLAG_CLEAR_SECRET != 0 {
            sodium_memzero(
                (*context).secret as *mut c_void,
                (*context).secretlen as usize,
            );
            (*context).secretlen = 0;
        }
    }

    store32_le(value.as_mut_ptr(), (*context).adlen);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    if !(*context).ad.is_null() {
        crypto_generichash_blake2b_update(
            &mut blake_hash,
            (*context).ad as *const u8,
            (*context).adlen as u64,
        );
    }

    crypto_generichash_blake2b_final(&mut blake_hash, blockhash, ARGON2_PREHASH_DIGEST_LENGTH);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_initialize(
    instance: *mut argon2_instance_t,
    context: *mut argon2_context,
) -> c_int {
    let mut blockhash: [u8; ARGON2_PREHASH_SEED_LENGTH] = [0; ARGON2_PREHASH_SEED_LENGTH];
    let result: c_int;

    if instance.is_null() || context.is_null() {
        return ARGON2_INCORRECT_PARAMETER;
    }

    /* 1. Memory allocation */
    (*instance).pseudo_rands =
        libc::malloc(core::mem::size_of::<u64>().wrapping_mul((*instance).segment_length as usize))
            as *mut u64;
    if (*instance).pseudo_rands.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    result = allocate_memory(&mut (*instance).region, (*instance).memory_blocks);
    if ARGON2_OK != result {
        argon2_free_instance(instance, (*context).flags as c_int);
        return result;
    }

    /* 2. Initial hashing */
    argon2_initial_hash(blockhash.as_mut_ptr(), context, (*instance).type_);
    /* Zeroing 8 extra bytes */
    sodium_memzero(
        blockhash.as_mut_ptr().add(ARGON2_PREHASH_DIGEST_LENGTH) as *mut c_void,
        ARGON2_PREHASH_SEED_LENGTH - ARGON2_PREHASH_DIGEST_LENGTH,
    );

    /* 3. Creating first blocks */
    argon2_fill_first_blocks(blockhash.as_mut_ptr(), instance);
    /* Clearing the hash */
    sodium_memzero(
        blockhash.as_mut_ptr() as *mut c_void,
        ARGON2_PREHASH_SEED_LENGTH,
    );

    ARGON2_OK
}

/* static int argon2_pick_best_implementation(void) */
fn argon2_pick_best_implementation() -> c_int {
    /* All SIMD paths compiled out (no HAVE_* macros); select the ref path. */
    unsafe {
        *fill_segment.0.get() = _sodium_argon2_fill_segment_ref;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn _crypto_pwhash_argon2_pick_best_implementation() -> c_int {
    argon2_pick_best_implementation()
}
