//! Translation of c_src/libsodium/crypto_pwhash/argon2/argon2-core.c

use crate::common::{load64_le, store32_le, store64_le};
use core::ffi::{c_int, c_void};

// ---- argon2.h / argon2-core.h constants ----
const ARGON2_BLOCK_SIZE: usize = 1024;
const ARGON2_QWORDS_IN_BLOCK: usize = ARGON2_BLOCK_SIZE / 8; // 128
const ARGON2_SYNC_POINTS: u32 = 4;
const ARGON2_VERSION_NUMBER: u32 = 0x13;
const ARGON2_PREHASH_DIGEST_LENGTH: usize = 64;
const ARGON2_PREHASH_SEED_LENGTH: usize = 72;

// argon2.h limits
const ARGON2_MIN_OUTLEN: u32 = 16;
const ARGON2_MAX_OUTLEN: u32 = 0xFFFFFFFF;
const ARGON2_MIN_PWD_LENGTH: u32 = 0;
const ARGON2_MAX_PWD_LENGTH: u32 = 0xFFFFFFFF;
const ARGON2_MIN_SALT_LENGTH: u32 = 8;
const ARGON2_MAX_SALT_LENGTH: u32 = 0xFFFFFFFF;
const ARGON2_MIN_SECRET: u32 = 0;
const ARGON2_MAX_SECRET: u32 = 0xFFFFFFFF;
const ARGON2_MIN_AD_LENGTH: u32 = 0;
const ARGON2_MAX_AD_LENGTH: u32 = 0xFFFFFFFF;
const ARGON2_MIN_LANES: u32 = 1;
const ARGON2_MAX_LANES: u32 = 0xFFFFFF;
const ARGON2_MIN_MEMORY: u32 = 2 * ARGON2_SYNC_POINTS; // 8
// ARGON2_MAX_MEMORY = min(0xFFFFFFFF, 1<<min(32, sizeof(void*)*8-10-1))
// 64-bit: min(32, 53) = 32 -> 1<<32 = 4294967296; min(0xFFFFFFFF, 4294967296) = 0xFFFFFFFF
const ARGON2_MAX_MEMORY: u32 = 0xFFFFFFFF;
const ARGON2_MIN_TIME: u32 = 1;
const ARGON2_MAX_TIME: u32 = 0xFFFFFFFF;
const ARGON2_MIN_THREADS: u32 = 1;
const ARGON2_MAX_THREADS: u32 = 0xFFFFFF;

const ARGON2_FLAG_CLEAR_PASSWORD: u32 = 1 << 0;
const ARGON2_FLAG_CLEAR_SECRET: u32 = 1 << 1;

// Error codes
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

// argon2_type
const Argon2_i: c_int = 1;
const Argon2_id: c_int = 2;

// ---- shared #[repr(C)] types (argon2-core.h / argon2.h) ----

#[repr(C)]
#[derive(Clone, Copy)]
struct block {
    v: [u64; ARGON2_QWORDS_IN_BLOCK],
}

#[repr(C)]
struct block_region {
    base: *mut c_void,
    memory: *mut block,
    size: usize,
}

#[repr(C)]
struct argon2_instance_t {
    region: *mut block_region,
    pseudo_rands: *mut u64,
    passes: u32,
    current_pass: u32,
    memory_blocks: u32,
    segment_length: u32,
    lane_length: u32,
    lanes: u32,
    threads: u32,
    type_: c_int, /* argon2_type */
    print_internals: c_int,
}

#[repr(C)]
struct argon2_position_t {
    pass: u32,
    lane: u32,
    slice: u8,
    index: u32,
}

#[repr(C)]
struct argon2_context {
    out: *mut u8,
    outlen: u32,
    pwd: *mut u8,
    pwdlen: u32,
    salt: *mut u8,
    saltlen: u32,
    secret: *mut u8,
    secretlen: u32,
    ad: *mut u8,
    adlen: u32,
    t_cost: u32,
    m_cost: u32,
    lanes: u32,
    threads: u32,
    flags: u32,
}

// crypto_generichash_blake2b_state (public opaque[384], CRYPTO_ALIGN(64))
#[repr(C, align(64))]
struct crypto_generichash_blake2b_state {
    opaque: [u8; 384],
}

// fill_segment function-pointer type
type fill_segment_fn =
    unsafe extern "C" fn(instance: *const argon2_instance_t, position: argon2_position_t);

extern "C" {
    // argon2-fill-block-ref.c -> _sodium_argon2_fill_segment_ref
    fn _sodium_argon2_fill_segment_ref(
        instance: *const argon2_instance_t,
        position: argon2_position_t,
    );
    // blake2b-long.c -> _sodium_blake2b_long
    fn _sodium_blake2b_long(
        pout: *mut c_void,
        outlen: usize,
        in_: *const c_void,
        inlen: usize,
    ) -> c_int;
    // crypto_generichash_blake2b (public API)
    fn crypto_generichash_blake2b_init(
        state: *mut crypto_generichash_blake2b_state,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b_update(
        state: *mut crypto_generichash_blake2b_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_generichash_blake2b_final(
        state: *mut crypto_generichash_blake2b_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;
    // exported helpers / libc
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

// (defined in blamka-round-ref.h -> used only here as inline helpers via
// argon2-core.h; the block helper functions are defined here.)

/* index_alpha() from argon2-core.h (static) */
unsafe fn index_alpha(
    instance: *const argon2_instance_t,
    position: *const argon2_position_t,
    pseudo_rand: u32,
    same_lane: c_int,
) -> u32 {
    let reference_area_size: u32;
    let mut relative_position: u64;
    let mut absolute_position: u64;
    let mut start_position: u32;

    if (*position).pass == 0 {
        /* First pass */
        if (*position).slice == 0 {
            /* First slice */
            reference_area_size = (*position).index.wrapping_sub(1); /* all but the previous */
        } else {
            if same_lane != 0 {
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
    start_position = 0;

    if (*position).pass != 0 {
        start_position = if (*position).slice as u32 == ARGON2_SYNC_POINTS - 1 {
            0
        } else {
            ((*position).slice as u32 + 1).wrapping_mul((*instance).segment_length)
        };
    }

    /* 1.2.6. Computing absolute position */
    absolute_position = (start_position as u64)
        .wrapping_add(relative_position)
        .wrapping_sub((*instance).lane_length as u64);
    absolute_position =
        absolute_position.wrapping_add((*instance).lane_length as u64 & (absolute_position >> 32));
    absolute_position as u32
}

/*****************Functions that work with the block (argon2-core.h inline)***/

unsafe fn init_block_value(b: *mut block, in_: u8) {
    memset(
        (*b).v.as_mut_ptr() as *mut c_void,
        in_ as c_int,
        core::mem::size_of::<[u64; ARGON2_QWORDS_IN_BLOCK]>(),
    );
}

unsafe fn copy_block(dst: *mut block, src: *const block) {
    core::ptr::copy_nonoverlapping(
        (*src).v.as_ptr(),
        (*dst).v.as_mut_ptr(),
        ARGON2_QWORDS_IN_BLOCK,
    );
}

unsafe fn xor_block(dst: *mut block, src: *const block) {
    let mut i: usize = 0;
    while i < ARGON2_QWORDS_IN_BLOCK {
        (*dst).v[i] ^= (*src).v[i];
        i += 1;
    }
}

// #[cfg]: x86_64 (not aarch64+NEON): fill_segment = argon2_fill_segment_ref
static mut fill_segment: fill_segment_fn = _sodium_argon2_fill_segment_ref;

unsafe fn load_block(dst: *mut block, input: *const c_void) {
    let mut i: usize = 0;
    while i < ARGON2_QWORDS_IN_BLOCK {
        (*dst).v[i] = load64_le((input as *const u8).add(i * core::mem::size_of::<u64>()));
        i += 1;
    }
}

unsafe fn store_block(output: *mut c_void, src: *const block) {
    let mut i: usize = 0;
    while i < ARGON2_QWORDS_IN_BLOCK {
        store64_le(
            (output as *mut u8).add(i * core::mem::size_of::<u64>()),
            (*src).v[i],
        );
        i += 1;
    }
}

/***************Memory allocators (static)*****************/

unsafe fn allocate_memory(region: *mut *mut block_region, m_cost: u32) -> c_int {
    let base: *mut c_void;
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

    // Reference build: no MAP_ANON+HAVE_MMAP, no HAVE_POSIX_MEMALIGN -> #else branch
    memory = core::ptr::null_mut();
    if memory_size.wrapping_add(63) < memory_size {
        base = core::ptr::null_mut();
        crate::plat::set_errno(crate::plat::ENOMEM);
    } else {
        let b = malloc(memory_size.wrapping_add(63));
        if !b.is_null() {
            let mut aligned: *mut u8 = (b as *mut u8).add(63);
            aligned = aligned.wrapping_sub((aligned as usize) & 63);
            memory = aligned as *mut block;
            base = b;
        } else {
            base = b;
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

/*********Memory functions (static)*/

unsafe fn free_memory(region: *mut block_region) {
    if !region.is_null() && !(*region).base.is_null() {
        // Reference build: no MAP_ANON+HAVE_MMAP -> free(region->base)
        free((*region).base);
    }
    free(region as *mut c_void);
}

unsafe fn argon2_free_instance(instance: *mut argon2_instance_t, _flags: c_int) {
    /* Deallocate the memory */
    free((*instance).pseudo_rands as *mut c_void);
    (*instance).pseudo_rands = core::ptr::null_mut();
    free_memory((*instance).region);
    (*instance).region = core::ptr::null_mut();
}

// argon2_finalize -> _sodium_argon2_finalize
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
                .add((*instance).lane_length as usize - 1),
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
            let mut blockhash_bytes: [u8; ARGON2_BLOCK_SIZE] = [0; ARGON2_BLOCK_SIZE];
            store_block(blockhash_bytes.as_mut_ptr() as *mut c_void, &blockhash);
            _sodium_blake2b_long(
                (*context).out as *mut c_void,
                (*context).outlen as usize,
                blockhash_bytes.as_ptr() as *const c_void,
                ARGON2_BLOCK_SIZE,
            );
            sodium_memzero(blockhash.v.as_mut_ptr() as *mut c_void, ARGON2_BLOCK_SIZE);
            sodium_memzero(blockhash_bytes.as_mut_ptr() as *mut c_void, ARGON2_BLOCK_SIZE);
        }

        argon2_free_instance(instance, (*context).flags as c_int);
    }
}

// argon2_fill_memory_blocks -> _sodium_argon2_fill_memory_blocks
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_fill_memory_blocks(
    instance: *mut argon2_instance_t,
    pass: u32,
) {
    let mut position: argon2_position_t = core::mem::zeroed();
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
            let pos = argon2_position_t {
                pass: position.pass,
                lane: position.lane,
                slice: position.slice,
                index: position.index,
            };
            (fill_segment)(instance as *const argon2_instance_t, pos);
            l = l.wrapping_add(1);
        }
        s = s.wrapping_add(1);
    }
}

// argon2_validate_inputs -> _sodium_argon2_validate_inputs
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_validate_inputs(
    context: *const argon2_context,
) -> c_int {
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

    if ARGON2_MAX_MEMORY < (*context).m_cost {
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

// error codes referenced above but declared here to keep them near use
const ARGON2_THREADS_TOO_FEW: c_int = -28;
const ARGON2_THREADS_TOO_MANY: c_int = -29;

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
                .add((l.wrapping_mul((*instance).lane_length) as usize) + 0),
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
                .add((l.wrapping_mul((*instance).lane_length) as usize) + 1),
            blockhash_bytes.as_ptr() as *const c_void,
        );
        l = l.wrapping_add(1);
    }
    sodium_memzero(blockhash_bytes.as_mut_ptr() as *mut c_void, ARGON2_BLOCK_SIZE);
}

unsafe fn argon2_initial_hash(
    blockhash: *mut u8,
    context: *mut argon2_context,
    type_: c_int,
) {
    let mut blake_hash: crypto_generichash_blake2b_state = core::mem::zeroed();
    let mut value: [u8; 4] = [0; 4]; /* sizeof(uint32_t) */

    if context.is_null() || blockhash.is_null() {
        return; /* LCOV_EXCL_LINE */
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

        /* LCOV_EXCL_START */
        if (*context).flags & ARGON2_FLAG_CLEAR_PASSWORD != 0 {
            sodium_memzero((*context).pwd as *mut c_void, (*context).pwdlen as usize);
            (*context).pwdlen = 0;
        }
        /* LCOV_EXCL_STOP */
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

    /* LCOV_EXCL_START */
    if !(*context).secret.is_null() {
        crypto_generichash_blake2b_update(
            &mut blake_hash,
            (*context).secret as *const u8,
            (*context).secretlen as u64,
        );

        if (*context).flags & ARGON2_FLAG_CLEAR_SECRET != 0 {
            sodium_memzero((*context).secret as *mut c_void, (*context).secretlen as usize);
            (*context).secretlen = 0;
        }
    }
    /* LCOV_EXCL_STOP */

    store32_le(value.as_mut_ptr(), (*context).adlen);
    crypto_generichash_blake2b_update(&mut blake_hash, value.as_ptr(), value.len() as u64);

    /* LCOV_EXCL_START */
    if !(*context).ad.is_null() {
        crypto_generichash_blake2b_update(
            &mut blake_hash,
            (*context).ad as *const u8,
            (*context).adlen as u64,
        );
    }
    /* LCOV_EXCL_STOP */

    crypto_generichash_blake2b_final(&mut blake_hash, blockhash, ARGON2_PREHASH_DIGEST_LENGTH);
}

// argon2_initialize -> _sodium_argon2_initialize
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_initialize(
    instance: *mut argon2_instance_t,
    context: *mut argon2_context,
) -> c_int {
    let mut blockhash: [u8; ARGON2_PREHASH_SEED_LENGTH] = [0; ARGON2_PREHASH_SEED_LENGTH];
    let result: c_int;

    if instance.is_null() || context.is_null() {
        return ARGON2_INCORRECT_PARAMETER; /* LCOV_EXCL_LINE */
    }

    /* 1. Memory allocation */
    (*instance).pseudo_rands = malloc(
        core::mem::size_of::<u64>().wrapping_mul((*instance).segment_length as usize),
    ) as *mut u64;
    if (*instance).pseudo_rands.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */
    }

    result = allocate_memory(&mut (*instance).region, (*instance).memory_blocks);
    if ARGON2_OK != result {
        argon2_free_instance(instance, (*context).flags as c_int); /* LCOV_EXCL_LINE */
        return result; /* LCOV_EXCL_LINE */
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

// argon2_pick_best_implementation (static)
// Reference build: no SIMD macros defined, x86_64 (not aarch64+NEON).
unsafe fn argon2_pick_best_implementation() -> c_int {
    /* LCOV_EXCL_START */
    // All #if SIMD branches take the false path; final #else selects ref.
    fill_segment = _sodium_argon2_fill_segment_ref;

    0
    /* LCOV_EXCL_STOP */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_pwhash_argon2_pick_best_implementation() -> c_int {
    argon2_pick_best_implementation()
}
