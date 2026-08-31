//! Translation of c_src/libsodium/crypto_pwhash/argon2/argon2.c

use core::ffi::{c_char, c_int, c_void};

// ---- argon2.h constants ----
const ARGON2_MAX_OUTLEN: usize = 0xFFFFFFFF;
const ARGON2_MAX_PWD_LENGTH: usize = 0xFFFFFFFF;
const ARGON2_MAX_SALT_LENGTH: usize = 0xFFFFFFFF;

// Error codes (argon2_error_codes)
const ARGON2_OK: c_int = 0;
const ARGON2_OUTPUT_TOO_LONG: c_int = -3;
const ARGON2_PWD_TOO_LONG: c_int = -5;
const ARGON2_SALT_TOO_LONG: c_int = -7;
const ARGON2_MEMORY_ALLOCATION_ERROR: c_int = -22;
const ARGON2_INCORRECT_TYPE: c_int = -26;
const ARGON2_ENCODING_FAIL: c_int = -31;
const ARGON2_DECODING_LENGTH_FAIL: c_int = -34;
const ARGON2_VERIFY_MISMATCH: c_int = -35;

const ARGON2_SYNC_POINTS: u32 = 4;
const ARGON2_DEFAULT_FLAGS: u32 = 0;

// argon2_type
const Argon2_i: c_int = 1;
const Argon2_id: c_int = 2;

// argon2_context (argon2.h). #[repr(C)] shared type.
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

extern "C" {
    // argon2-core.c (renamed by quirks.h)
    fn _sodium_argon2_validate_inputs(context: *const argon2_context) -> c_int;
    fn _sodium_argon2_initialize(
        instance: *mut c_void,
        context: *mut argon2_context,
    ) -> c_int;
    fn _sodium_argon2_fill_memory_blocks(instance: *mut c_void, pass: u32);
    fn _sodium_argon2_finalize(context: *const argon2_context, instance: *mut c_void);
    // argon2-encoding.c (renamed by quirks.h)
    fn _sodium_argon2_encode_string(
        dst: *mut c_char,
        dst_len: usize,
        ctx: *mut argon2_context,
        type_: c_int,
    ) -> c_int;
    fn _sodium_argon2_decode_string(
        ctx: *mut argon2_context,
        str_: *const c_char,
        type_: c_int,
    ) -> c_int;
    // exported helpers
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1: *const c_void, b2: *const c_void, len: usize) -> c_int;
    // libc
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

// argon2_instance_t is defined in argon2-core.c; here we only need storage of
// identical layout to hold one on the stack and pass its address across FFI.
// argon2-core.h: block_region*, uint64_t*, then 9 u32-ish fields + type + int.
#[repr(C)]
struct argon2_instance_t {
    region: *mut c_void,       /* block_region * */
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

// argon2_ctx -> _sodium_argon2_ctx
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_ctx(
    context: *mut argon2_context,
    type_: c_int,
) -> c_int {
    /* 1. Validate all inputs */
    let mut result: c_int = _sodium_argon2_validate_inputs(context);
    let memory_blocks: u32;
    let segment_length: u32;
    let mut pass: u32;
    let mut instance: argon2_instance_t = core::mem::zeroed();

    if ARGON2_OK != result {
        return result; /* LCOV_EXCL_LINE */
    }

    if type_ != Argon2_id && type_ != Argon2_i {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }

    /* 2. Align memory size */
    let mut mb: u32 = (*context).m_cost;

    if mb < 2u32.wrapping_mul(ARGON2_SYNC_POINTS).wrapping_mul((*context).lanes) {
        mb = 2u32
            .wrapping_mul(ARGON2_SYNC_POINTS)
            .wrapping_mul((*context).lanes); /* LCOV_EXCL_LINE */
    }

    segment_length =
        mb / ((*context).lanes.wrapping_mul(ARGON2_SYNC_POINTS));
    /* Ensure that all segments have equal length */
    memory_blocks =
        segment_length.wrapping_mul((*context).lanes.wrapping_mul(ARGON2_SYNC_POINTS));

    instance.region = core::ptr::null_mut();
    instance.passes = (*context).t_cost;
    instance.current_pass = !0u32;
    instance.memory_blocks = memory_blocks;
    instance.segment_length = segment_length;
    instance.lane_length = segment_length.wrapping_mul(ARGON2_SYNC_POINTS);
    instance.lanes = (*context).lanes;
    instance.threads = (*context).threads;
    instance.type_ = type_;

    /* 3. Initialization */
    result = _sodium_argon2_initialize(
        &mut instance as *mut argon2_instance_t as *mut c_void,
        context,
    );

    if ARGON2_OK != result {
        return result; /* LCOV_EXCL_LINE */
    }

    /* 4. Filling memory */
    pass = 0;
    while pass < instance.passes {
        _sodium_argon2_fill_memory_blocks(
            &mut instance as *mut argon2_instance_t as *mut c_void,
            pass,
        );
        pass = pass.wrapping_add(1);
    }

    /* 5. Finalization */
    _sodium_argon2_finalize(
        context,
        &mut instance as *mut argon2_instance_t as *mut c_void,
    );

    ARGON2_OK
}

// argon2_hash -> _sodium_argon2_hash
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
    type_: c_int,
) -> c_int {
    let mut context: argon2_context = core::mem::zeroed();
    let result: c_int;
    let out: *mut u8;

    if hash != core::ptr::null_mut() {
        randombytes_buf(hash, hashlen);
    }

    if pwdlen > ARGON2_MAX_PWD_LENGTH {
        return ARGON2_PWD_TOO_LONG; /* LCOV_EXCL_LINE */
    }

    if hashlen > ARGON2_MAX_OUTLEN {
        return ARGON2_OUTPUT_TOO_LONG; /* LCOV_EXCL_LINE */
    }

    if saltlen > ARGON2_MAX_SALT_LENGTH {
        return ARGON2_SALT_TOO_LONG; /* LCOV_EXCL_LINE */
    }

    out = malloc(hashlen) as *mut u8;
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
        core::ptr::copy_nonoverlapping(out, hash as *mut u8, hashlen);
    }

    sodium_memzero(out as *mut c_void, hashlen);
    free(out as *mut c_void);

    ARGON2_OK
}

// argon2i_hash_encoded -> _sodium_argon2i_hash_encoded
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

// argon2i_hash_raw -> _sodium_argon2i_hash_raw
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

// argon2id_hash_encoded -> _sodium_argon2id_hash_encoded
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

// argon2id_hash_raw -> _sodium_argon2id_hash_raw
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

// argon2_verify -> _sodium_argon2_verify
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
    type_: c_int,
) -> c_int {
    let mut ctx: argon2_context = core::mem::zeroed();
    let out: *mut u8;
    let decode_result: c_int;
    let mut ret: c_int;
    let encoded_len: usize;

    memset(
        &mut ctx as *mut argon2_context as *mut c_void,
        0,
        core::mem::size_of::<argon2_context>(),
    );

    ctx.pwd = core::ptr::null_mut();
    ctx.pwdlen = 0;
    ctx.secret = core::ptr::null_mut();
    ctx.secretlen = 0;

    /* max values, to be updated in argon2_decode_string */
    encoded_len = strlen(encoded);
    if encoded_len > u32::MAX as usize {
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
        core::ptr::null_mut(),
        0,
        type_,
    );

    free(ctx.ad as *mut c_void);
    free(ctx.salt as *mut c_void);

    if ret == ARGON2_OK
        && sodium_memcmp(out as *const c_void, ctx.out as *const c_void, ctx.outlen as usize) != 0
    {
        ret = ARGON2_VERIFY_MISMATCH;
    }
    free(out as *mut c_void);
    free(ctx.out as *mut c_void);

    ret
}

// argon2i_verify -> _sodium_argon2i_verify
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2i_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
) -> c_int {
    _sodium_argon2_verify(encoded, pwd, pwdlen, Argon2_i)
}

// argon2id_verify -> _sodium_argon2id_verify
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2id_verify(
    encoded: *const c_char,
    pwd: *const c_void,
    pwdlen: usize,
) -> c_int {
    _sodium_argon2_verify(encoded, pwd, pwdlen, Argon2_id)
}
