//! Translation of `crypto_pwhash/argon2/argon2.c`
//!
//! Argon2 source code package
//!
//! Written by Daniel Dinu and Dmitry Khovratovich, 2015
//!
//! This work is licensed under a Creative Commons CC0 1.0 License/Waiver.

use crate::common::*;
use core::ffi::{c_char, c_int, c_uint, c_void};
use core::mem::MaybeUninit;
use core::ptr;

/* ---------------------------------------------------------------- */
/* argon2.h constants                                                */
/* ---------------------------------------------------------------- */

/* Number of synchronization points between lanes per pass */
pub const ARGON2_SYNC_POINTS: u32 = 4;

/* Minimum and maximum digest size in bytes */
pub const ARGON2_MAX_OUTLEN: u32 = 0xFFFFFFFF;

/* Minimum and maximum password length in bytes */
pub const ARGON2_MAX_PWD_LENGTH: u32 = 0xFFFFFFFF;

/* Minimum and maximum salt length in bytes */
pub const ARGON2_MAX_SALT_LENGTH: u32 = 0xFFFFFFFF;

pub const ARGON2_DEFAULT_FLAGS: u32 = 0;

/* Error codes (enum Argon2_ErrorCodes) */
pub const ARGON2_OK: c_int = 0;
pub const ARGON2_OUTPUT_TOO_LONG: c_int = -3;
pub const ARGON2_PWD_TOO_LONG: c_int = -5;
pub const ARGON2_SALT_TOO_LONG: c_int = -7;
pub const ARGON2_MEMORY_ALLOCATION_ERROR: c_int = -22;
pub const ARGON2_INCORRECT_TYPE: c_int = -26;
pub const ARGON2_ENCODING_FAIL: c_int = -31;
pub const ARGON2_DECODING_LENGTH_FAIL: c_int = -34;
pub const ARGON2_VERIFY_MISMATCH: c_int = -35;

/* enum Argon2_type { Argon2_i = 1, Argon2_id = 2 } -- gcc picks `unsigned int` */
pub const Argon2_i: c_uint = 1;
pub const Argon2_id: c_uint = 2;

/// `typedef struct Argon2_Context { ... } argon2_context;`
///
/// Exact layout from `argon2.h` (96 bytes, `flags` at offset 92).
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

/// `typedef struct Argon2_instance_t { ... } argon2_instance_t;`
///
/// Exact layout from `argon2-core.h` (56 bytes, `type` at offset 44,
/// `print_internals` at offset 48).  `block_region *` is kept opaque.
#[repr(C)]
pub struct argon2_instance_t {
    pub region: *mut c_void, /* block_region * */
    pub pseudo_rands: *mut u64,
    pub passes: u32,
    pub current_pass: u32,
    pub memory_blocks: u32,
    pub segment_length: u32,
    pub lane_length: u32,
    pub lanes: u32,
    pub threads: u32,
    pub type_: c_uint, /* argon2_type */
    pub print_internals: c_int,
}

extern "C" {
    /* argon2-core.c */
    fn _sodium_argon2_validate_inputs(context: *const argon2_context) -> c_int;
    fn _sodium_argon2_initialize(
        instance: *mut argon2_instance_t,
        context: *mut argon2_context,
    ) -> c_int;
    fn _sodium_argon2_fill_memory_blocks(instance: *mut argon2_instance_t, pass: u32);
    fn _sodium_argon2_finalize(context: *const argon2_context, instance: *mut argon2_instance_t);

    /* argon2-encoding.c */
    fn _sodium_argon2_encode_string(
        dst: *mut c_char,
        dst_len: usize,
        ctx: *mut argon2_context,
        type_: c_uint,
    ) -> c_int;
    fn _sodium_argon2_decode_string(
        ctx: *mut argon2_context,
        str_: *const c_char,
        type_: c_uint,
    ) -> c_int;

    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);

    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int;

    /* libc */
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

/* int argon2_ctx(argon2_context *context, argon2_type type) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_ctx(
    context: *mut argon2_context,
    type_: c_uint,
) -> c_int {
    /* 1. Validate all inputs */
    let mut result: c_int = _sodium_argon2_validate_inputs(context);
    let mut memory_blocks: u32;
    let segment_length: u32;
    let mut pass: u32;
    let mut instance_uninit: MaybeUninit<argon2_instance_t> = MaybeUninit::uninit();
    let instance: *mut argon2_instance_t = instance_uninit.as_mut_ptr();

    if ARGON2_OK != result {
        return result; /* LCOV_EXCL_LINE */
    }

    if type_ != Argon2_id && type_ != Argon2_i {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }

    /* 2. Align memory size */
    /* Minimum memory_blocks = 8L blocks, where L is the number of lanes */
    memory_blocks = (*context).m_cost;

    if memory_blocks
        < (2u32.wrapping_mul(ARGON2_SYNC_POINTS)).wrapping_mul((*context).lanes)
    {
        /* LCOV_EXCL_LINE */
        memory_blocks = (2u32.wrapping_mul(ARGON2_SYNC_POINTS)).wrapping_mul((*context).lanes);
    }

    segment_length = memory_blocks / ((*context).lanes.wrapping_mul(ARGON2_SYNC_POINTS));
    /* Ensure that all segments have equal length */
    memory_blocks = segment_length.wrapping_mul((*context).lanes.wrapping_mul(ARGON2_SYNC_POINTS));

    ptr::addr_of_mut!((*instance).region).write(ptr::null_mut());
    ptr::addr_of_mut!((*instance).passes).write((*context).t_cost);
    ptr::addr_of_mut!((*instance).current_pass).write(!0u32);
    ptr::addr_of_mut!((*instance).memory_blocks).write(memory_blocks);
    ptr::addr_of_mut!((*instance).segment_length).write(segment_length);
    ptr::addr_of_mut!((*instance).lane_length)
        .write(segment_length.wrapping_mul(ARGON2_SYNC_POINTS));
    ptr::addr_of_mut!((*instance).lanes).write((*context).lanes);
    ptr::addr_of_mut!((*instance).threads).write((*context).threads);
    ptr::addr_of_mut!((*instance).type_).write(type_);

    /* 3. Initialization: Hashing inputs, allocating memory, filling first
     * blocks
     */
    result = _sodium_argon2_initialize(instance, context);

    if ARGON2_OK != result {
        return result; /* LCOV_EXCL_LINE */
    }

    /* 4. Filling memory */
    pass = 0;
    while pass < (*instance).passes {
        _sodium_argon2_fill_memory_blocks(instance, pass);
        pass = pass.wrapping_add(1);
    }

    /* 5. Finalization */
    _sodium_argon2_finalize(context, instance);

    ARGON2_OK
}

/* int argon2_hash(...) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_hash(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hash: *mut c_void,
    hashlen: usize,
    encoded: *mut c_char,
    encodedlen: usize,
    type_: c_uint,
) -> c_int {
    let mut context: argon2_context;
    let result: c_int;
    let out: *mut u8;

    if !hash.is_null() {
        randombytes_buf(hash, hashlen);
    }

    if pwdlen > ARGON2_MAX_PWD_LENGTH as usize {
        return ARGON2_PWD_TOO_LONG; /* LCOV_EXCL_LINE */
    }

    if hashlen > ARGON2_MAX_OUTLEN as usize {
        return ARGON2_OUTPUT_TOO_LONG; /* LCOV_EXCL_LINE */
    }

    if saltlen > ARGON2_MAX_SALT_LENGTH as usize {
        return ARGON2_SALT_TOO_LONG; /* LCOV_EXCL_LINE */
    }

    out = malloc(hashlen) as *mut u8;
    if out.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */
    }

    context = argon2_context {
        out: out,
        outlen: hashlen as u32,
        pwd: pwd as *mut u8,
        pwdlen: pwdlen as u32,
        salt: salt as *mut u8,
        saltlen: saltlen as u32,
        secret: ptr::null_mut(),
        secretlen: 0,
        ad: ptr::null_mut(),
        adlen: 0,
        t_cost: t_cost,
        m_cost: m_cost,
        lanes: parallelism,
        threads: parallelism,
        flags: ARGON2_DEFAULT_FLAGS,
    };

    result = _sodium_argon2_ctx(&mut context, type_);

    /* LCOV_EXCL_START */
    if result != ARGON2_OK {
        sodium_memzero(out as *mut c_void, hashlen);
        free(out as *mut c_void);
        return result;
    }
    /* LCOV_EXCL_STOP */

    /* if encoding requested, write it */
    if !encoded.is_null() && encodedlen != 0 {
        if _sodium_argon2_encode_string(encoded, encodedlen, &mut context, type_) != ARGON2_OK {
            /* LCOV_EXCL_START */
            sodium_memzero(out as *mut c_void, hashlen);
            sodium_memzero(encoded as *mut c_void, encodedlen);
            free(out as *mut c_void);
            return ARGON2_ENCODING_FAIL;
            /* LCOV_EXCL_STOP */
        }
    }

    /* if raw hash requested, write it */
    if !hash.is_null() {
        memcpy(hash as *mut u8, out as *const u8, hashlen);
    }

    sodium_memzero(out as *mut c_void, hashlen);
    free(out as *mut c_void);

    ARGON2_OK
}

/* int argon2i_hash_encoded(...) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2i_hash_encoded(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hashlen: usize,
    encoded: *mut c_char,
    encodedlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        ptr::null_mut(),
        hashlen,
        encoded,
        encodedlen,
        Argon2_i,
    )
}

/* int argon2i_hash_raw(...) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2i_hash_raw(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hash: *mut c_void,
    hashlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        hash,
        hashlen,
        ptr::null_mut(),
        0,
        Argon2_i,
    )
}

/* int argon2id_hash_encoded(...) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2id_hash_encoded(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hashlen: usize,
    encoded: *mut c_char,
    encodedlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        ptr::null_mut(),
        hashlen,
        encoded,
        encodedlen,
        Argon2_id,
    )
}

/* int argon2id_hash_raw(...) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2id_hash_raw(
    t_cost: u32,
    m_cost: u32,
    parallelism: u32,
    pwd: *const c_void,
    pwdlen: usize,
    salt: *const c_void,
    saltlen: usize,
    hash: *mut c_void,
    hashlen: usize,
) -> c_int {
    _sodium_argon2_hash(
        t_cost,
        m_cost,
        parallelism,
        pwd,
        pwdlen,
        salt,
        saltlen,
        hash,
        hashlen,
        ptr::null_mut(),
        0,
        Argon2_id,
    )
}

/* int argon2_verify(const char *encoded, const void *pwd, const size_t pwdlen,
                     argon2_type type) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
    type_: c_uint,
) -> c_int {
    let mut ctx: argon2_context = core::mem::zeroed();
    let out: *mut u8;
    let decode_result: c_int;
    let mut ret: c_int;
    let encoded_len: usize;

    /* memset(&ctx, 0, sizeof ctx) -- done by core::mem::zeroed() above */

    ctx.pwd = ptr::null_mut();
    ctx.pwdlen = 0;
    ctx.secret = ptr::null_mut();
    ctx.secretlen = 0;

    /* max values, to be updated in argon2_decode_string */
    encoded_len = strlen(encoded);
    if encoded_len > 0xFFFF_FFFFusize {
        return ARGON2_DECODING_LENGTH_FAIL; /* LCOV_EXCL_LINE */
    }
    ctx.adlen = encoded_len as u32;
    ctx.saltlen = encoded_len as u32;
    ctx.outlen = encoded_len as u32;

    ctx.ad = malloc(ctx.adlen as usize) as *mut u8;
    ctx.salt = malloc(ctx.saltlen as usize) as *mut u8;
    ctx.out = malloc(ctx.outlen as usize) as *mut u8;
    /* LCOV_EXCL_START */
    if ctx.out.is_null() || ctx.salt.is_null() || ctx.ad.is_null() {
        free(ctx.ad as *mut c_void);
        free(ctx.salt as *mut c_void);
        free(ctx.out as *mut c_void);
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    /* LCOV_EXCL_STOP */
    out = malloc(ctx.outlen as usize) as *mut u8;
    if out.is_null() {
        /* LCOV_EXCL_START */
        free(ctx.ad as *mut c_void);
        free(ctx.salt as *mut c_void);
        free(ctx.out as *mut c_void);
        return ARGON2_MEMORY_ALLOCATION_ERROR;
        /* LCOV_EXCL_STOP */
    }

    decode_result = _sodium_argon2_decode_string(&mut ctx, encoded, type_);
    if decode_result != ARGON2_OK {
        free(ctx.ad as *mut c_void);
        free(ctx.salt as *mut c_void);
        free(ctx.out as *mut c_void);
        free(out as *mut c_void);
        return decode_result;
    }

    ret = _sodium_argon2_hash(
        ctx.t_cost,
        ctx.m_cost,
        ctx.threads,
        pwd,
        pwdlen,
        ctx.salt as *const c_void,
        ctx.saltlen as usize,
        out as *mut c_void,
        ctx.outlen as usize,
        ptr::null_mut(),
        0,
        type_,
    );

    free(ctx.ad as *mut c_void);
    free(ctx.salt as *mut c_void);

    if ret == ARGON2_OK
        && sodium_memcmp(
            out as *const c_void,
            ctx.out as *const c_void,
            ctx.outlen as usize,
        ) != 0
    {
        ret = ARGON2_VERIFY_MISMATCH;
    }
    free(out as *mut c_void);
    free(ctx.out as *mut c_void);

    ret
}

/* int argon2i_verify(const char *encoded, const void *pwd, const size_t pwdlen) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2i_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
) -> c_int {
    _sodium_argon2_verify(encoded, pwd, pwdlen, Argon2_i)
}

/* int argon2id_verify(const char *encoded, const void *pwd, const size_t pwdlen) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2id_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
) -> c_int {
    _sodium_argon2_verify(encoded, pwd, pwdlen, Argon2_id)
}
