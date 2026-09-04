//! Translation of `crypto_pwhash/argon2/argon2.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::common::memcpy;
use crate::randombytes::randombytes_buf;
use crate::sodium_utils::{sodium_memcmp, sodium_memzero};

use super::argon2_core::*;
use super::argon2_encoding::{_sodium_argon2_decode_string, _sodium_argon2_encode_string};

/* Local strlen with identical semantics for a NUL-terminated C string. */
unsafe fn strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_ctx(
    context: *mut argon2_context,
    type_: argon2_type,
) -> c_int {
    /* 1. Validate all inputs */
    let mut result: c_int = _sodium_argon2_validate_inputs(context);
    let mut memory_blocks: u32;
    let segment_length: u32;
    let mut pass: u32;
    let mut instance: argon2_instance_t = core::mem::zeroed();

    if ARGON2_OK != result {
        return result;
    }

    if type_ != Argon2_id && type_ != Argon2_i {
        return ARGON2_INCORRECT_TYPE;
    }

    /* 2. Align memory size */
    memory_blocks = (*context).m_cost;

    if memory_blocks < 2 * ARGON2_SYNC_POINTS * (*context).lanes {
        memory_blocks = 2 * ARGON2_SYNC_POINTS * (*context).lanes;
    }

    segment_length = memory_blocks / ((*context).lanes * ARGON2_SYNC_POINTS);
    /* Ensure that all segments have equal length */
    memory_blocks = segment_length * ((*context).lanes * ARGON2_SYNC_POINTS);

    instance.region = core::ptr::null_mut();
    instance.passes = (*context).t_cost;
    instance.current_pass = !0u32;
    instance.memory_blocks = memory_blocks;
    instance.segment_length = segment_length;
    instance.lane_length = segment_length * ARGON2_SYNC_POINTS;
    instance.lanes = (*context).lanes;
    instance.threads = (*context).threads;
    instance.type_ = type_;

    /* 3. Initialization */
    result = _sodium_argon2_initialize(&mut instance, context);

    if ARGON2_OK != result {
        return result;
    }

    /* 4. Filling memory */
    pass = 0;
    while pass < instance.passes {
        _sodium_argon2_fill_memory_blocks(&mut instance, pass);
        pass += 1;
    }

    /* 5. Finalization */
    _sodium_argon2_finalize(context, &mut instance);

    ARGON2_OK
}

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
    let mut context: argon2_context = core::mem::zeroed();
    let result: c_int;
    let out: *mut u8;

    if !hash.is_null() {
        randombytes_buf(hash, hashlen);
    }

    if pwdlen > ARGON2_MAX_PWD_LENGTH as usize {
        return ARGON2_PWD_TOO_LONG;
    }

    if hashlen > ARGON2_MAX_OUTLEN as usize {
        return ARGON2_OUTPUT_TOO_LONG;
    }

    if saltlen > ARGON2_MAX_SALT_LENGTH as usize {
        return ARGON2_SALT_TOO_LONG;
    }

    out = libc::malloc(hashlen) as *mut u8;
    if out.is_null() {
        return ARGON2_MEMORY_ALLOCATION_ERROR;
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

    result = _sodium_argon2_ctx(&mut context, type_);

    if result != ARGON2_OK {
        sodium_memzero(out as *mut c_void, hashlen);
        libc::free(out as *mut c_void);
        return result;
    }

    /* if encoding requested, write it */
    if !encoded.is_null() && encodedlen != 0 {
        if _sodium_argon2_encode_string(encoded, encodedlen, &mut context, type_) != ARGON2_OK {
            sodium_memzero(out as *mut c_void, hashlen);
            sodium_memzero(encoded as *mut c_void, encodedlen);
            libc::free(out as *mut c_void);
            return ARGON2_ENCODING_FAIL;
        }
    }

    /* if raw hash requested, write it */
    if !hash.is_null() {
        memcpy(hash as *mut u8, out, hashlen);
    }

    sodium_memzero(out as *mut c_void, hashlen);
    libc::free(out as *mut c_void);

    ARGON2_OK
}

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
        core::ptr::null_mut(),
        hashlen,
        encoded,
        encodedlen,
        Argon2_i,
    )
}

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
        core::ptr::null_mut(),
        0,
        Argon2_i,
    )
}

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
        core::ptr::null_mut(),
        hashlen,
        encoded,
        encodedlen,
        Argon2_id,
    )
}

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
        core::ptr::null_mut(),
        0,
        Argon2_id,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
    type_: argon2_type,
) -> c_int {
    let mut ctx: argon2_context = core::mem::zeroed();
    let out: *mut u8;
    let decode_result: c_int;
    let mut ret: c_int;
    let encoded_len: usize;

    /* memset(&ctx, 0, sizeof ctx) already covered by zeroed() */

    ctx.pwd = core::ptr::null_mut();
    ctx.pwdlen = 0;
    ctx.secret = core::ptr::null_mut();
    ctx.secretlen = 0;

    /* max values, to be updated in argon2_decode_string */
    encoded_len = strlen(encoded);
    if encoded_len > u32::MAX as usize {
        return ARGON2_DECODING_LENGTH_FAIL;
    }
    ctx.adlen = encoded_len as u32;
    ctx.saltlen = encoded_len as u32;
    ctx.outlen = encoded_len as u32;

    ctx.ad = libc::malloc(ctx.adlen as usize) as *mut u8;
    ctx.salt = libc::malloc(ctx.saltlen as usize) as *mut u8;
    ctx.out = libc::malloc(ctx.outlen as usize) as *mut u8;
    if ctx.out.is_null() || ctx.salt.is_null() || ctx.ad.is_null() {
        libc::free(ctx.ad as *mut c_void);
        libc::free(ctx.salt as *mut c_void);
        libc::free(ctx.out as *mut c_void);
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }
    out = libc::malloc(ctx.outlen as usize) as *mut u8;
    if out.is_null() {
        libc::free(ctx.ad as *mut c_void);
        libc::free(ctx.salt as *mut c_void);
        libc::free(ctx.out as *mut c_void);
        return ARGON2_MEMORY_ALLOCATION_ERROR;
    }

    decode_result = _sodium_argon2_decode_string(&mut ctx, encoded, type_);
    if decode_result != ARGON2_OK {
        libc::free(ctx.ad as *mut c_void);
        libc::free(ctx.salt as *mut c_void);
        libc::free(ctx.out as *mut c_void);
        libc::free(out as *mut c_void);
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
        core::ptr::null_mut(),
        0,
        type_,
    );

    libc::free(ctx.ad as *mut c_void);
    libc::free(ctx.salt as *mut c_void);

    if ret == ARGON2_OK
        && sodium_memcmp(out as *const c_void, ctx.out as *const c_void, ctx.outlen as usize) != 0
    {
        ret = ARGON2_VERIFY_MISMATCH;
    }
    libc::free(out as *mut c_void);
    libc::free(ctx.out as *mut c_void);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2i_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
) -> c_int {
    _sodium_argon2_verify(encoded, pwd, pwdlen, Argon2_i)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2id_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
) -> c_int {
    _sodium_argon2_verify(encoded, pwd, pwdlen, Argon2_id)
}
