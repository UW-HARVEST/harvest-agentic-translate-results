//! Translation of `crypto_pwhash/argon2/argon2.c` together with the shared
//! types, constants and enums of `crypto_pwhash/argon2/argon2.h`.
//!
//! `include/sodium/private/quirks.h` renames every non-static argon2 symbol,
//! e.g. `argon2_ctx` -> `_sodium_argon2_ctx`.

use core::ffi::{c_char, c_int, c_void};

use crate::common::{free, malloc, strlen};
use crate::crypto_pwhash::argon2::argon2_core::{
    _sodium_argon2_fill_memory_blocks, _sodium_argon2_finalize, _sodium_argon2_initialize,
    _sodium_argon2_validate_inputs, argon2_instance_t,
};
use crate::crypto_pwhash::argon2::argon2_encoding::{
    _sodium_argon2_decode_string, _sodium_argon2_encode_string,
};
use crate::randombytes::randombytes_buf;
use crate::sodium::utils::{sodium_memcmp, sodium_memzero};

// ---------------------------------------------------------------------------
// argon2.h -- Argon2 input parameter restrictions
// ---------------------------------------------------------------------------

/// `#define ARGON2_MIN_LANES UINT32_C(1)`
pub const ARGON2_MIN_LANES: u32 = 1;
/// `#define ARGON2_MAX_LANES UINT32_C(0xFFFFFF)`
pub const ARGON2_MAX_LANES: u32 = 0x00FF_FFFF;

/// `#define ARGON2_MIN_THREADS UINT32_C(1)`
pub const ARGON2_MIN_THREADS: u32 = 1;
/// `#define ARGON2_MAX_THREADS UINT32_C(0xFFFFFF)`
pub const ARGON2_MAX_THREADS: u32 = 0x00FF_FFFF;

/// `#define ARGON2_SYNC_POINTS UINT32_C(4)`
pub const ARGON2_SYNC_POINTS: u32 = 4;

/// `#define ARGON2_MIN_OUTLEN UINT32_C(16)`
pub const ARGON2_MIN_OUTLEN: u32 = 16;
/// `#define ARGON2_MAX_OUTLEN UINT32_C(0xFFFFFFFF)`
pub const ARGON2_MAX_OUTLEN: u32 = 0xFFFF_FFFF;

/// `#define ARGON2_MIN_MEMORY (2 * ARGON2_SYNC_POINTS)`
pub const ARGON2_MIN_MEMORY: u32 = 2 * ARGON2_SYNC_POINTS;

/// `ARGON2_MIN(UINT32_C(32), (sizeof(void *) * CHAR_BIT - 10 - 1))` == 32
pub const ARGON2_MAX_MEMORY_BITS: u32 = 32;
/// `ARGON2_MIN(UINT32_C(0xFFFFFFFF), UINT64_C(1) << ARGON2_MAX_MEMORY_BITS)`
///
/// The `?:` mixes `uint32_t` and `uint64_t`, so the macro has type
/// `uint64_t` with the value `0xFFFFFFFF`.
pub const ARGON2_MAX_MEMORY: u64 = 0xFFFF_FFFF;

/// `#define ARGON2_MIN_TIME UINT32_C(1)`
pub const ARGON2_MIN_TIME: u32 = 1;
/// `#define ARGON2_MAX_TIME UINT32_C(0xFFFFFFFF)`
pub const ARGON2_MAX_TIME: u32 = 0xFFFF_FFFF;

/// `#define ARGON2_MIN_PWD_LENGTH UINT32_C(0)`
pub const ARGON2_MIN_PWD_LENGTH: u32 = 0;
/// `#define ARGON2_MAX_PWD_LENGTH UINT32_C(0xFFFFFFFF)`
pub const ARGON2_MAX_PWD_LENGTH: u32 = 0xFFFF_FFFF;

/// `#define ARGON2_MIN_AD_LENGTH UINT32_C(0)`
pub const ARGON2_MIN_AD_LENGTH: u32 = 0;
/// `#define ARGON2_MAX_AD_LENGTH UINT32_C(0xFFFFFFFF)`
pub const ARGON2_MAX_AD_LENGTH: u32 = 0xFFFF_FFFF;

/// `#define ARGON2_MIN_SALT_LENGTH UINT32_C(8)`
pub const ARGON2_MIN_SALT_LENGTH: u32 = 8;
/// `#define ARGON2_MAX_SALT_LENGTH UINT32_C(0xFFFFFFFF)`
pub const ARGON2_MAX_SALT_LENGTH: u32 = 0xFFFF_FFFF;

/// `#define ARGON2_MIN_SECRET UINT32_C(0)`
pub const ARGON2_MIN_SECRET: u32 = 0;
/// `#define ARGON2_MAX_SECRET UINT32_C(0xFFFFFFFF)`
pub const ARGON2_MAX_SECRET: u32 = 0xFFFF_FFFF;

/// `#define ARGON2_FLAG_CLEAR_PASSWORD (UINT32_C(1) << 0)`
pub const ARGON2_FLAG_CLEAR_PASSWORD: u32 = 1u32 << 0;
/// `#define ARGON2_FLAG_CLEAR_SECRET (UINT32_C(1) << 1)`
pub const ARGON2_FLAG_CLEAR_SECRET: u32 = 1u32 << 1;
/// `#define ARGON2_DEFAULT_FLAGS (UINT32_C(0))`
pub const ARGON2_DEFAULT_FLAGS: u32 = 0;

// ---------------------------------------------------------------------------
// argon2.h -- `typedef enum Argon2_ErrorCodes { ... } argon2_error_codes;`
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// argon2.h -- `typedef struct Argon2_Context { ... } argon2_context;`
// ---------------------------------------------------------------------------

/// `sizeof(argon2_context) == 96`; offsets 0, 8, 16, 24, 32, 40, 48, 56, 64,
/// 72, 76, 80, 84, 88, 92 (verified with a C probe).
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

impl argon2_context {
    /// `memset(&ctx, 0, sizeof ctx)`
    pub const fn zeroed() -> Self {
        argon2_context {
            out: core::ptr::null_mut(),
            outlen: 0,
            pwd: core::ptr::null_mut(),
            pwdlen: 0,
            salt: core::ptr::null_mut(),
            saltlen: 0,
            secret: core::ptr::null_mut(),
            secretlen: 0,
            ad: core::ptr::null_mut(),
            adlen: 0,
            t_cost: 0,
            m_cost: 0,
            lanes: 0,
            threads: 0,
            flags: 0,
        }
    }
}

/// `typedef enum Argon2_type { Argon2_i = 1, Argon2_id = 2 } argon2_type;`
///
/// The enum fits in an `int`, so the C ABI passes it as `int`.
pub type argon2_type = c_int;
pub const Argon2_i: argon2_type = 1;
pub const Argon2_id: argon2_type = 2;

// ---------------------------------------------------------------------------
// argon2.c
// ---------------------------------------------------------------------------

/// `int argon2_ctx(argon2_context *context, argon2_type type)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_ctx(
    context: *mut argon2_context,
    type_: argon2_type,
) -> c_int {
    /* 1. Validate all inputs */
    let mut result: c_int = unsafe { _sodium_argon2_validate_inputs(context) };
    let mut memory_blocks: u32;
    let segment_length: u32;
    let mut pass: u32;
    let mut instance: argon2_instance_t = argon2_instance_t::zeroed();

    if ARGON2_OK != result {
        return result; /* LCOV_EXCL_LINE */
    }

    if type_ != Argon2_id && type_ != Argon2_i {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }

    /* 2. Align memory size */
    /* Minimum memory_blocks = 8L blocks, where L is the number of lanes */
    memory_blocks = unsafe { (*context).m_cost };

    if memory_blocks
        < (2u32
            .wrapping_mul(ARGON2_SYNC_POINTS)
            .wrapping_mul(unsafe { (*context).lanes }))
    {
        memory_blocks = 2u32
            .wrapping_mul(ARGON2_SYNC_POINTS)
            .wrapping_mul(unsafe { (*context).lanes }); /* LCOV_EXCL_LINE */
    }

    segment_length = memory_blocks
        / (unsafe { (*context).lanes }.wrapping_mul(ARGON2_SYNC_POINTS));
    /* Ensure that all segments have equal length */
    memory_blocks = segment_length
        .wrapping_mul(unsafe { (*context).lanes }.wrapping_mul(ARGON2_SYNC_POINTS));

    instance.region = core::ptr::null_mut();
    instance.passes = unsafe { (*context).t_cost };
    instance.current_pass = !0u32;
    instance.memory_blocks = memory_blocks;
    instance.segment_length = segment_length;
    instance.lane_length = segment_length.wrapping_mul(ARGON2_SYNC_POINTS);
    instance.lanes = unsafe { (*context).lanes };
    instance.threads = unsafe { (*context).threads };
    instance.type_ = type_;

    /* 3. Initialization: Hashing inputs, allocating memory, filling first
     * blocks
     */
    result = unsafe { _sodium_argon2_initialize(&mut instance, context) };

    if ARGON2_OK != result {
        return result; /* LCOV_EXCL_LINE */
    }

    /* 4. Filling memory */
    pass = 0;
    while pass < instance.passes {
        unsafe { _sodium_argon2_fill_memory_blocks(&mut instance, pass) };
        pass = pass.wrapping_add(1);
    }

    /* 5. Finalization */
    unsafe { _sodium_argon2_finalize(context, &mut instance) };

    ARGON2_OK
}

/// `int argon2_hash(...)`
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
    type_: argon2_type,
) -> c_int {
    let mut context: argon2_context = argon2_context::zeroed();
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

    out = unsafe { malloc(hashlen) } as *mut u8;
    if out.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */
    }

    context.out = out;
    context.outlen = hashlen as u32;
    context.pwd = pwd as *mut u8;
    context.pwdlen = pwdlen as u32;
    context.salt = salt as *mut u8;
    context.saltlen = saltlen as u32;
    context.secret = core::ptr::null_mut();
    context.secretlen = 0;
    context.ad = core::ptr::null_mut();
    context.adlen = 0;
    context.t_cost = t_cost;
    context.m_cost = m_cost;
    context.lanes = parallelism;
    context.threads = parallelism;
    context.flags = ARGON2_DEFAULT_FLAGS;

    result = unsafe { _sodium_argon2_ctx(&mut context, type_) };

    /* LCOV_EXCL_START */
    if result != ARGON2_OK {
        unsafe { sodium_memzero(out as *mut c_void, hashlen) };
        unsafe { free(out as *mut c_void) };
        return result;
    }
    /* LCOV_EXCL_STOP */

    /* if encoding requested, write it */
    if !encoded.is_null() && encodedlen != 0 {
        if unsafe { _sodium_argon2_encode_string(encoded, encodedlen, &mut context, type_) }
            != ARGON2_OK
        {
            /* LCOV_EXCL_START */
            unsafe { sodium_memzero(out as *mut c_void, hashlen) };
            unsafe { sodium_memzero(encoded as *mut c_void, encodedlen) };
            unsafe { free(out as *mut c_void) };
            return ARGON2_ENCODING_FAIL;
            /* LCOV_EXCL_STOP */
        }
    }

    /* if raw hash requested, write it */
    if !hash.is_null() {
        unsafe { crate::common::memcpy(hash as *mut u8, out as *const u8, hashlen) };
    }

    unsafe { sodium_memzero(out as *mut c_void, hashlen) };
    unsafe { free(out as *mut c_void) };

    ARGON2_OK
}

/// `int argon2i_hash_encoded(...)`
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
    unsafe {
        _sodium_argon2_hash(
            t_cost,
            m_cost,
            parallelism,
            pwd,
            pwdlen,
            salt,
            saltlen,
            core::ptr::null_mut(),
            hashlen,
            encoded,
            encodedlen,
            Argon2_i,
        )
    }
}

/// `int argon2i_hash_raw(...)`
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
    unsafe {
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
            core::ptr::null_mut(),
            0,
            Argon2_i,
        )
    }
}

/// `int argon2id_hash_encoded(...)`
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
    unsafe {
        _sodium_argon2_hash(
            t_cost,
            m_cost,
            parallelism,
            pwd,
            pwdlen,
            salt,
            saltlen,
            core::ptr::null_mut(),
            hashlen,
            encoded,
            encodedlen,
            Argon2_id,
        )
    }
}

/// `int argon2id_hash_raw(...)`
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
    unsafe {
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
            core::ptr::null_mut(),
            0,
            Argon2_id,
        )
    }
}

/// `int argon2_verify(const char *encoded, const void *pwd, const size_t pwdlen,
///                    argon2_type type)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
    type_: argon2_type,
) -> c_int {
    let mut ctx: argon2_context;
    let out: *mut u8;
    let decode_result: c_int;
    let mut ret: c_int;
    let encoded_len: usize;

    ctx = argon2_context::zeroed();

    ctx.pwd = core::ptr::null_mut();
    ctx.pwdlen = 0;
    ctx.secret = core::ptr::null_mut();
    ctx.secretlen = 0;

    /* max values, to be updated in argon2_decode_string */
    encoded_len = unsafe { strlen(encoded) };
    if encoded_len > u32::MAX as usize {
        return ARGON2_DECODING_LENGTH_FAIL; /* LCOV_EXCL_LINE */
    }
    ctx.adlen = encoded_len as u32;
    ctx.saltlen = encoded_len as u32;
    ctx.outlen = encoded_len as u32;

    ctx.ad = unsafe { malloc(ctx.adlen as usize) } as *mut u8;
    ctx.salt = unsafe { malloc(ctx.saltlen as usize) } as *mut u8;
    ctx.out = unsafe { malloc(ctx.outlen as usize) } as *mut u8;
    /* LCOV_EXCL_START */
    if ctx.out.is_null() || ctx.salt.is_null() || ctx.ad.is_null() {
        unsafe { free(ctx.ad as *mut c_void) };
        unsafe { free(ctx.salt as *mut c_void) };
        unsafe { free(ctx.out as *mut c_void) };
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    /* LCOV_EXCL_STOP */
    out = unsafe { malloc(ctx.outlen as usize) } as *mut u8;
    if out.is_null() {
        /* LCOV_EXCL_START */
        unsafe { free(ctx.ad as *mut c_void) };
        unsafe { free(ctx.salt as *mut c_void) };
        unsafe { free(ctx.out as *mut c_void) };
        return ARGON2_MEMORY_ALLOCATION_ERROR;
        /* LCOV_EXCL_STOP */
    }

    decode_result = unsafe { _sodium_argon2_decode_string(&mut ctx, encoded, type_) };
    if decode_result != ARGON2_OK {
        unsafe { free(ctx.ad as *mut c_void) };
        unsafe { free(ctx.salt as *mut c_void) };
        unsafe { free(ctx.out as *mut c_void) };
        unsafe { free(out as *mut c_void) };
        return decode_result;
    }

    ret = unsafe {
        _sodium_argon2_hash(
            ctx.t_cost,
            ctx.m_cost,
            ctx.threads,
            pwd,
            pwdlen,
            ctx.salt as *const c_void,
            ctx.saltlen as usize,
            out as *mut c_void,
            ctx.outlen as usize,
            core::ptr::null_mut(),
            0,
            type_,
        )
    };

    unsafe { free(ctx.ad as *mut c_void) };
    unsafe { free(ctx.salt as *mut c_void) };

    if ret == ARGON2_OK
        && unsafe {
            sodium_memcmp(
                out as *const c_void,
                ctx.out as *const c_void,
                ctx.outlen as usize,
            )
        } != 0
    {
        ret = ARGON2_VERIFY_MISMATCH;
    }
    unsafe { free(out as *mut c_void) };
    unsafe { free(ctx.out as *mut c_void) };

    ret
}

/// `int argon2i_verify(const char *encoded, const void *pwd, const size_t pwdlen)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2i_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
) -> c_int {
    unsafe { _sodium_argon2_verify(encoded, pwd, pwdlen, Argon2_i) }
}

/// `int argon2id_verify(const char *encoded, const void *pwd, const size_t pwdlen)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2id_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
) -> c_int {
    unsafe { _sodium_argon2_verify(encoded, pwd, pwdlen, Argon2_id) }
}
