//! Translation of `crypto_pwhash/argon2/argon2-core.c`.
//!
//! Exports (after the `private/quirks.h` renaming):
//!   * `_crypto_pwhash_argon2_pick_best_implementation`
//!   * `_sodium_argon2_fill_memory_blocks`
//!   * `_sodium_argon2_finalize`
//!   * `_sodium_argon2_initialize`
//!   * `_sodium_argon2_validate_inputs`
//!
//! The reference build has no `config.h`, therefore `HAVE_SYS_MMAN_H`,
//! `HAVE_MMAP` and `HAVE_POSIX_MEMALIGN` are all undefined: `<sys/mman.h>` is
//! not included (so `MAP_ANON` never gets defined either) and
//! `allocate_memory()` takes the plain `malloc(memory_size + 63)` +
//! manual-64-byte-alignment fallback, with `free_memory()` calling `free()`.
//! Likewise `__wasm_simd128__`, `HAVE_AVX512FINTRIN_H`, `HAVE_AVX2INTRIN_H`,
//! `HAVE_TMMINTRIN_H`, `HAVE_SMMINTRIN_H`, `HAVE_EMMINTRIN_H`, `__aarch64__`
//! and `__ARM_NEON` are undefined, so `fill_segment` is always
//! `argon2_fill_segment_ref` and `argon2_pick_best_implementation()` reduces to
//! a plain assignment.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* ---------------------------------------------------------------- argon2.h */

/* Argon2 input parameter restrictions */
const ARGON2_MIN_LANES: u32 = 1;
const ARGON2_MAX_LANES: u32 = 0x00FF_FFFF;

const ARGON2_MIN_THREADS: u32 = 1;
const ARGON2_MAX_THREADS: u32 = 0x00FF_FFFF;

const ARGON2_SYNC_POINTS: u32 = 4;

const ARGON2_MIN_OUTLEN: u32 = 16;
const ARGON2_MAX_OUTLEN: u32 = 0xFFFF_FFFF;

const ARGON2_MIN_MEMORY: u32 = 2 * ARGON2_SYNC_POINTS; /* 2 blocks per slice */

/* ARGON2_MAX_MEMORY = ARGON2_MIN(UINT32_C(0xFFFFFFFF),
 *                                UINT64_C(1) << ARGON2_MAX_MEMORY_BITS)
 * with ARGON2_MAX_MEMORY_BITS = ARGON2_MIN(32, sizeof(void*) * CHAR_BIT - 11)
 *                             = min(32, 53) = 32.
 * The `?:` operands are `unsigned int` and `unsigned long long`, so the result
 * has type `unsigned long long` and value 0xFFFFFFFF. */
const ARGON2_MAX_MEMORY: u64 = 0xFFFF_FFFF;

const ARGON2_MIN_TIME: u32 = 1;
const ARGON2_MAX_TIME: u32 = 0xFFFF_FFFF;

const ARGON2_MIN_PWD_LENGTH: u32 = 0;
const ARGON2_MAX_PWD_LENGTH: u32 = 0xFFFF_FFFF;

const ARGON2_MIN_AD_LENGTH: u32 = 0;
const ARGON2_MAX_AD_LENGTH: u32 = 0xFFFF_FFFF;

const ARGON2_MIN_SALT_LENGTH: u32 = 8;
const ARGON2_MAX_SALT_LENGTH: u32 = 0xFFFF_FFFF;

const ARGON2_MIN_SECRET: u32 = 0;
const ARGON2_MAX_SECRET: u32 = 0xFFFF_FFFF;

const ARGON2_FLAG_CLEAR_PASSWORD: u32 = 1u32 << 0;
const ARGON2_FLAG_CLEAR_SECRET: u32 = 1u32 << 1;

/* enum Argon2_ErrorCodes */
const ARGON2_OK: c_int = 0;
const ARGON2_OUTPUT_PTR_NULL: c_int = -1;
const ARGON2_OUTPUT_TOO_SHORT: c_int = -2;
const ARGON2_OUTPUT_TOO_LONG: c_int = -3;
const ARGON2_PWD_TOO_SHORT: c_int = -4;
const ARGON2_PWD_TOO_LONG: c_int = -5;
const ARGON2_SALT_TOO_SHORT: c_int = -6;
const ARGON2_SALT_TOO_LONG: c_int = -7;
const ARGON2_AD_TOO_SHORT: c_int = -8;
const ARGON2_AD_TOO_LONG: c_int = -9;
const ARGON2_SECRET_TOO_SHORT: c_int = -10;
const ARGON2_SECRET_TOO_LONG: c_int = -11;
const ARGON2_TIME_TOO_SMALL: c_int = -12;
const ARGON2_TIME_TOO_LARGE: c_int = -13;
const ARGON2_MEMORY_TOO_LITTLE: c_int = -14;
const ARGON2_MEMORY_TOO_MUCH: c_int = -15;
const ARGON2_LANES_TOO_FEW: c_int = -16;
const ARGON2_LANES_TOO_MANY: c_int = -17;
const ARGON2_PWD_PTR_MISMATCH: c_int = -18;
const ARGON2_SALT_PTR_MISMATCH: c_int = -19;
const ARGON2_SECRET_PTR_MISMATCH: c_int = -20;
const ARGON2_AD_PTR_MISMATCH: c_int = -21;
const ARGON2_MEMORY_ALLOCATION_ERROR: c_int = -22;
const ARGON2_INCORRECT_PARAMETER: c_int = -25;
const ARGON2_THREADS_TOO_FEW: c_int = -28;
const ARGON2_THREADS_TOO_MANY: c_int = -29;

/* typedef struct Argon2_Context (argon2.h); size 96, align 8 */
#[repr(C)]
pub struct argon2_context {
    pub out: *mut u8,   /* output array */
    pub outlen: u32,    /* digest length */
    pub pwd: *mut u8,   /* password array */
    pub pwdlen: u32,    /* password length */
    pub salt: *mut u8,  /* salt array */
    pub saltlen: u32,   /* salt length */
    pub secret: *mut u8, /* key array */
    pub secretlen: u32, /* key length */
    pub ad: *mut u8,    /* associated data array */
    pub adlen: u32,     /* associated data length */
    pub t_cost: u32,    /* number of passes */
    pub m_cost: u32,    /* amount of memory requested (KB) */
    pub lanes: u32,     /* number of lanes */
    pub threads: u32,   /* maximum number of threads */
    pub flags: u32,     /* array of bool options */
}

/* typedef enum Argon2_type { Argon2_i = 1, Argon2_id = 2 } argon2_type; */
pub type argon2_type = c_int;

/* ----------------------------------------------------------- argon2-core.h */

/* enum argon2_ctx_constants */
const ARGON2_VERSION_NUMBER: u32 = 0x13;
const ARGON2_BLOCK_SIZE: usize = 1024;
const ARGON2_QWORDS_IN_BLOCK: usize = ARGON2_BLOCK_SIZE / 8;
const ARGON2_PREHASH_DIGEST_LENGTH: usize = 64;
const ARGON2_PREHASH_SEED_LENGTH: usize = 72;

/* typedef struct block_ { uint64_t v[ARGON2_QWORDS_IN_BLOCK]; } block; */
#[repr(C)]
pub struct block {
    pub v: [u64; ARGON2_QWORDS_IN_BLOCK],
}

/* typedef struct block_region_ (argon2-core.h); size 24, align 8 */
#[repr(C)]
pub struct block_region {
    pub base: *mut c_void,
    pub memory: *mut block,
    pub size: usize,
}

/* typedef struct Argon2_instance_t (argon2-core.h); size 56, align 8 */
#[repr(C)]
pub struct argon2_instance_t {
    pub region: *mut block_region, /* Memory region pointer */
    pub pseudo_rands: *mut u64,
    pub passes: u32, /* Number of passes */
    pub current_pass: u32,
    pub memory_blocks: u32, /* Number of blocks in memory */
    pub segment_length: u32,
    pub lane_length: u32,
    pub lanes: u32,
    pub threads: u32,
    pub type_: argon2_type,
    pub print_internals: c_int, /* whether to print the memory blocks */
}

/* typedef struct Argon2_position_t (argon2-core.h); size 16, align 4 */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct argon2_position_t {
    pub pass: u32,
    pub lane: u32,
    pub slice: u8,
    pub index: u32,
}

/* typedef void (*fill_segment_fn)(const argon2_instance_t *instance,
 *                                 argon2_position_t        position); */
type fill_segment_fn = unsafe extern "C" fn(*const argon2_instance_t, argon2_position_t);

/* --------------------------------------------- static inline block helpers */

/* static inline void copy_block(block *dst, const block *src) */
#[inline(always)]
unsafe fn copy_block(dst: *mut block, src: *const block) {
    memcpy(
        core::ptr::addr_of_mut!((*dst).v) as *mut u8,
        core::ptr::addr_of!((*src).v) as *const u8,
        8usize * ARGON2_QWORDS_IN_BLOCK,
    );
}

/* static inline void xor_block(block *dst, const block *src) */
#[inline(always)]
unsafe fn xor_block(dst: *mut block, src: *const block) {
    let mut i: usize = 0;
    while i < ARGON2_QWORDS_IN_BLOCK {
        (*dst).v[i] ^= (*src).v[i];
        i += 1;
    }
}

/* ------------------------------------------------------ external functions */

/* `errno` is set by allocate_memory() on the overflow path. */
const ENOMEM: c_int = 12;

extern "C" {
    /* <stdlib.h> */
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    /* <errno.h> */
    fn __errno_location() -> *mut c_int;

    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);

    /* crypto_pwhash/argon2/blake2b-long.c */
    fn _sodium_blake2b_long(
        pout: *mut c_void,
        outlen: usize,
        in_: *const c_void,
        inlen: usize,
    ) -> c_int;

    /* crypto_pwhash/argon2/argon2-fill-block-ref.c */
    fn _sodium_argon2_fill_segment_ref(
        instance: *const argon2_instance_t,
        position: argon2_position_t,
    );

    /* crypto_generichash/blake2b/ref/generichash_blake2b.c -- `state` is a
     * `crypto_generichash_blake2b_state *`; passed as an opaque pointer here so
     * that the declaration matches the other translation units. */
    fn crypto_generichash_blake2b_init(
        state: *mut c_void,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b_update(
        state: *mut c_void,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_generichash_blake2b_final(state: *mut c_void, out: *mut u8, outlen: usize) -> c_int;
}

/* `typedef struct CRYPTO_ALIGN(64) crypto_generichash_blake2b_state` from
 * include/sodium/crypto_generichash_blake2b.h: sizeof == 384, _Alignof == 64. */
#[repr(C, align(64))]
struct crypto_generichash_blake2b_state {
    opaque: [u8; 384],
}

/* ------------------------------------------------------------------ bodies */

/* static fill_segment_fn fill_segment = argon2_fill_segment_ref; */
static mut fill_segment: fill_segment_fn = _sodium_argon2_fill_segment_ref;

/* static void load_block(block *dst, const void *input) */
unsafe fn load_block(dst: *mut block, input: *const c_void) {
    let mut i: usize = 0;
    while i < ARGON2_QWORDS_IN_BLOCK {
        (*dst).v[i] = load64_le((input as *const u8).add(i * 8));
        i += 1;
    }
}

/* static void store_block(void *output, const block *src) */
unsafe fn store_block(output: *mut c_void, src: *const block) {
    let mut i: usize = 0;
    while i < ARGON2_QWORDS_IN_BLOCK {
        store64_le((output as *mut u8).add(i * 8), (*src).v[i]);
        i += 1;
    }
}

/***************Memory allocators*****************/
/* static int allocate_memory(block_region **region, uint32_t m_cost) */
unsafe fn allocate_memory(region: *mut *mut block_region, m_cost: u32) -> c_int {
    let mut base: *mut c_void;
    let mut memory: *mut block;
    let memory_size: usize;

    if region.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */
    }
    memory_size = core::mem::size_of::<block>().wrapping_mul(m_cost as usize);
    if m_cost == 0 || memory_size / (m_cost as usize) != core::mem::size_of::<block>() {
        return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */
    }
    *region = malloc(core::mem::size_of::<block_region>()) as *mut block_region;
    if (*region).is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */
    }
    (**region).memory = core::ptr::null_mut();
    (**region).base = core::ptr::null_mut();

    memory = core::ptr::null_mut();
    if memory_size.wrapping_add(63) < memory_size {
        base = core::ptr::null_mut();
        *__errno_location() = ENOMEM;
    } else {
        base = malloc(memory_size.wrapping_add(63));
        if !base.is_null() {
            let mut aligned: *mut u8 = (base as *mut u8).add(63);
            aligned = aligned.wrapping_sub((aligned as usize) & 63);
            memory = aligned as *mut block;
        }
    }
    if base.is_null() {
        /* LCOV_EXCL_START */
        free(*region as *mut c_void);
        *region = core::ptr::null_mut();
        return ARGON2_MEMORY_ALLOCATION_ERROR;
        /* LCOV_EXCL_STOP */
    }
    (**region).base = base;
    (**region).memory = memory;
    (**region).size = memory_size;

    ARGON2_OK
}

/*********Memory functions*/

/* static void free_memory(block_region *region) */
unsafe fn free_memory(region: *mut block_region) {
    if !region.is_null() && !(*region).base.is_null() {
        free((*region).base);
    }
    free(region as *mut c_void);
}

/* static void argon2_free_instance(argon2_instance_t *instance, int flags) */
unsafe fn argon2_free_instance(instance: *mut argon2_instance_t, _flags: c_int) {
    /* Deallocate the memory */
    free((*instance).pseudo_rands as *mut c_void);
    (*instance).pseudo_rands = core::ptr::null_mut();
    free_memory((*instance).region);
    (*instance).region = core::ptr::null_mut();
}

/* void argon2_finalize(const argon2_context *context,
 *                      argon2_instance_t *instance) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_finalize(
    context: *const argon2_context,
    instance: *mut argon2_instance_t,
) {
    if !context.is_null() && !instance.is_null() {
        let mut blockhash = block {
            v: [0u64; ARGON2_QWORDS_IN_BLOCK],
        };
        let mut l: u32;

        copy_block(
            &mut blockhash,
            (*(*instance).region)
                .memory
                .add((*instance).lane_length.wrapping_sub(1) as usize),
        );

        /* XOR the last blocks */
        l = 1;
        while l < (*instance).lanes {
            let last_block_in_lane: u32 = l
                .wrapping_mul((*instance).lane_length)
                .wrapping_add((*instance).lane_length.wrapping_sub(1));
            xor_block(
                &mut blockhash,
                (*(*instance).region).memory.add(last_block_in_lane as usize),
            );
            l = l.wrapping_add(1);
        }

        /* Hash the result */
        {
            let mut blockhash_bytes: [u8; ARGON2_BLOCK_SIZE] = [0u8; ARGON2_BLOCK_SIZE];
            store_block(blockhash_bytes.as_mut_ptr() as *mut c_void, &blockhash);
            _sodium_blake2b_long(
                (*context).out as *mut c_void,
                (*context).outlen as usize,
                blockhash_bytes.as_ptr() as *const c_void,
                ARGON2_BLOCK_SIZE,
            );
            sodium_memzero(
                core::ptr::addr_of_mut!(blockhash.v) as *mut c_void,
                ARGON2_BLOCK_SIZE,
            ); /* clear blockhash */
            sodium_memzero(
                blockhash_bytes.as_mut_ptr() as *mut c_void,
                ARGON2_BLOCK_SIZE,
            ); /* clear blockhash_bytes */
        }

        argon2_free_instance(instance, (*context).flags as c_int);
    }
}

/* void argon2_fill_memory_blocks(argon2_instance_t *instance, uint32_t pass) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_fill_memory_blocks(
    instance: *mut argon2_instance_t,
    pass: u32,
) {
    let mut position = argon2_position_t {
        pass: 0,
        lane: 0,
        slice: 0,
        index: 0,
    };
    let mut l: u32;
    let mut s: u32;

    if instance.is_null() || (*instance).lanes == 0 {
        return; /* LCOV_EXCL_LINE */
    }

    position.pass = pass;
    s = 0;
    while s < ARGON2_SYNC_POINTS {
        position.slice = s as u8;
        l = 0;
        while l < (*instance).lanes {
            position.lane = l;
            position.index = 0;
            let f: fill_segment_fn = fill_segment;
            f(instance as *const argon2_instance_t, position);
            l = l.wrapping_add(1);
        }
        s = s.wrapping_add(1);
    }
}

/* int argon2_validate_inputs(const argon2_context *context) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_validate_inputs(context: *const argon2_context) -> c_int {
    /* LCOV_EXCL_START */
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
    /* LCOV_EXCL_STOP */

    ARGON2_OK
}

/* static void argon2_fill_first_blocks(uint8_t *blockhash,
 *                                      const argon2_instance_t *instance) */
unsafe fn argon2_fill_first_blocks(blockhash: *mut u8, instance: *const argon2_instance_t) {
    let mut l: u32;
    /* Make the first and second block in each lane as G(H0||i||0) or
       G(H0||i||1) */
    let mut blockhash_bytes: [u8; ARGON2_BLOCK_SIZE] = [0u8; ARGON2_BLOCK_SIZE];
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
                .add(l.wrapping_mul((*instance).lane_length).wrapping_add(0) as usize),
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
                .add(l.wrapping_mul((*instance).lane_length).wrapping_add(1) as usize),
            blockhash_bytes.as_ptr() as *const c_void,
        );
        l = l.wrapping_add(1);
    }
    sodium_memzero(
        blockhash_bytes.as_mut_ptr() as *mut c_void,
        ARGON2_BLOCK_SIZE,
    );
}

/* static void argon2_initial_hash(uint8_t *blockhash, argon2_context *context,
 *                                 argon2_type type) */
unsafe fn argon2_initial_hash(
    blockhash: *mut u8,
    context: *mut argon2_context,
    type_: argon2_type,
) {
    let mut BlakeHash = crypto_generichash_blake2b_state { opaque: [0u8; 384] };
    let mut value: [u8; 4 /* sizeof(uint32_t) */] = [0u8; 4];

    if context.is_null() || blockhash.is_null() {
        return; /* LCOV_EXCL_LINE */
    }

    crypto_generichash_blake2b_init(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        core::ptr::null(),
        0usize,
        ARGON2_PREHASH_DIGEST_LENGTH,
    );

    store32_le(value.as_mut_ptr(), (*context).lanes);
    crypto_generichash_blake2b_update(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        value.as_ptr(),
        4 as c_ulonglong,
    );

    store32_le(value.as_mut_ptr(), (*context).outlen);
    crypto_generichash_blake2b_update(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        value.as_ptr(),
        4 as c_ulonglong,
    );

    store32_le(value.as_mut_ptr(), (*context).m_cost);
    crypto_generichash_blake2b_update(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        value.as_ptr(),
        4 as c_ulonglong,
    );

    store32_le(value.as_mut_ptr(), (*context).t_cost);
    crypto_generichash_blake2b_update(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        value.as_ptr(),
        4 as c_ulonglong,
    );

    store32_le(value.as_mut_ptr(), ARGON2_VERSION_NUMBER);
    crypto_generichash_blake2b_update(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        value.as_ptr(),
        4 as c_ulonglong,
    );

    store32_le(value.as_mut_ptr(), type_ as u32);
    crypto_generichash_blake2b_update(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        value.as_ptr(),
        4 as c_ulonglong,
    );

    store32_le(value.as_mut_ptr(), (*context).pwdlen);
    crypto_generichash_blake2b_update(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        value.as_ptr(),
        4 as c_ulonglong,
    );

    if !(*context).pwd.is_null() {
        crypto_generichash_blake2b_update(
            &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
            (*context).pwd as *const u8,
            (*context).pwdlen as c_ulonglong,
        );

        /* LCOV_EXCL_START */
        if (*context).flags & ARGON2_FLAG_CLEAR_PASSWORD != 0 {
            sodium_memzero((*context).pwd as *mut c_void, (*context).pwdlen as usize);
            (*context).pwdlen = 0;
        }
        /* LCOV_EXCL_STOP */
    }

    store32_le(value.as_mut_ptr(), (*context).saltlen);
    crypto_generichash_blake2b_update(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        value.as_ptr(),
        4 as c_ulonglong,
    );

    if !(*context).salt.is_null() {
        crypto_generichash_blake2b_update(
            &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
            (*context).salt as *const u8,
            (*context).saltlen as c_ulonglong,
        );
    }

    store32_le(value.as_mut_ptr(), (*context).secretlen);
    crypto_generichash_blake2b_update(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        value.as_ptr(),
        4 as c_ulonglong,
    );

    /* LCOV_EXCL_START */
    if !(*context).secret.is_null() {
        crypto_generichash_blake2b_update(
            &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
            (*context).secret as *const u8,
            (*context).secretlen as c_ulonglong,
        );

        if (*context).flags & ARGON2_FLAG_CLEAR_SECRET != 0 {
            sodium_memzero((*context).secret as *mut c_void, (*context).secretlen as usize);
            (*context).secretlen = 0;
        }
    }
    /* LCOV_EXCL_STOP */

    store32_le(value.as_mut_ptr(), (*context).adlen);
    crypto_generichash_blake2b_update(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        value.as_ptr(),
        4 as c_ulonglong,
    );

    /* LCOV_EXCL_START */
    if !(*context).ad.is_null() {
        crypto_generichash_blake2b_update(
            &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
            (*context).ad as *const u8,
            (*context).adlen as c_ulonglong,
        );
    }
    /* LCOV_EXCL_STOP */

    crypto_generichash_blake2b_final(
        &mut BlakeHash as *mut crypto_generichash_blake2b_state as *mut c_void,
        blockhash,
        ARGON2_PREHASH_DIGEST_LENGTH,
    );
}

/* int argon2_initialize(argon2_instance_t *instance, argon2_context *context) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_initialize(
    instance: *mut argon2_instance_t,
    context: *mut argon2_context,
) -> c_int {
    let mut blockhash: [u8; ARGON2_PREHASH_SEED_LENGTH] = [0u8; ARGON2_PREHASH_SEED_LENGTH];
    let result: c_int;

    if instance.is_null() || context.is_null() {
        return ARGON2_INCORRECT_PARAMETER; /* LCOV_EXCL_LINE */
    }

    /* 1. Memory allocation */

    (*instance).pseudo_rands =
        malloc(8usize.wrapping_mul((*instance).segment_length as usize)) as *mut u64;
    if (*instance).pseudo_rands.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */
    }

    result = allocate_memory(
        core::ptr::addr_of_mut!((*instance).region),
        (*instance).memory_blocks,
    );
    if ARGON2_OK != result {
        argon2_free_instance(instance, (*context).flags as c_int); /* LCOV_EXCL_LINE */
        return result; /* LCOV_EXCL_LINE */
    }

    /* 2. Initial hashing */
    /* H_0 + 8 extra bytes to produce the first blocks */
    /* uint8_t blockhash[ARGON2_PREHASH_SEED_LENGTH]; */
    /* Hashing all inputs */
    argon2_initial_hash(blockhash.as_mut_ptr(), context, (*instance).type_);
    /* Zeroing 8 extra bytes */
    sodium_memzero(
        blockhash.as_mut_ptr().add(ARGON2_PREHASH_DIGEST_LENGTH) as *mut c_void,
        ARGON2_PREHASH_SEED_LENGTH - ARGON2_PREHASH_DIGEST_LENGTH,
    );

    /* 3. Creating first blocks, we always have at least two blocks in a slice
     */
    argon2_fill_first_blocks(blockhash.as_mut_ptr(), instance);
    /* Clearing the hash */
    sodium_memzero(
        blockhash.as_mut_ptr() as *mut c_void,
        ARGON2_PREHASH_SEED_LENGTH,
    );

    ARGON2_OK
}

/* static int argon2_pick_best_implementation(void) */
unsafe fn argon2_pick_best_implementation() -> c_int {
    /* LCOV_EXCL_START */
    fill_segment = _sodium_argon2_fill_segment_ref;

    0
    /* LCOV_EXCL_STOP */
}

/* int _crypto_pwhash_argon2_pick_best_implementation(void) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_pwhash_argon2_pick_best_implementation() -> c_int {
    argon2_pick_best_implementation()
}
