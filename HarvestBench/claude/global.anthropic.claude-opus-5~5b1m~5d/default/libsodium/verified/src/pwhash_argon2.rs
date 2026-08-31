//! Translation of:
//!   crypto_pwhash/argon2/pwhash_argon2i.c
//!   crypto_pwhash/argon2/pwhash_argon2id.c
//!   crypto_pwhash/crypto_pwhash.c

#![allow(non_camel_case_types, non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_void};

use crate::csys::{calloc, free, memset, set_errno, strlen, strncmp, EINVAL};

/// libsodium's SIZE_MAX equivalent for this target (usize::MAX).
const SODIUM_SIZE_MAX: usize = usize::MAX;

/// EFBIG is not declared in csys.rs; declare it locally.
const EFBIG: c_int = 27;

// ---------------------------------------------------------------------
// argon2.h types (repr(C), matching crypto_pwhash/argon2/argon2.h)
// ---------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
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

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum argon2_type {
    Argon2_i = 1,
    Argon2_id = 2,
}

const ARGON2_OK: c_int = 0;
const ARGON2_VERIFY_MISMATCH: c_int = -35;

// ---------------------------------------------------------------------
// Cross-module externs
// ---------------------------------------------------------------------

extern "C" {
    #[link_name = "_sodium_argon2i_hash_encoded"]
    fn argon2i_hash_encoded(
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
    ) -> c_int;

    #[link_name = "_sodium_argon2id_hash_encoded"]
    fn argon2id_hash_encoded(
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
    ) -> c_int;

    #[link_name = "_sodium_argon2i_hash_raw"]
    fn argon2i_hash_raw(
        t_cost: u32,
        m_cost: u32,
        parallelism: u32,
        pwd: *const c_void,
        pwdlen: usize,
        salt: *const c_void,
        saltlen: usize,
        hash: *mut c_void,
        hashlen: usize,
    ) -> c_int;

    #[link_name = "_sodium_argon2id_hash_raw"]
    fn argon2id_hash_raw(
        t_cost: u32,
        m_cost: u32,
        parallelism: u32,
        pwd: *const c_void,
        pwdlen: usize,
        salt: *const c_void,
        saltlen: usize,
        hash: *mut c_void,
        hashlen: usize,
    ) -> c_int;

    #[link_name = "_sodium_argon2i_verify"]
    fn argon2i_verify(encoded: *const c_char, pwd: *const c_void, pwdlen: usize) -> c_int;

    #[link_name = "_sodium_argon2id_verify"]
    fn argon2id_verify(encoded: *const c_char, pwd: *const c_void, pwdlen: usize) -> c_int;

    // `type_` is a plain `int` here (as in C); see `_sodium_argon2_ctx`.
    #[link_name = "_sodium_argon2_decode_string"]
    fn argon2_decode_string(
        ctx: *mut argon2_context,
        str_: *const c_char,
        type_: c_int,
    ) -> c_int;

    fn randombytes_buf(buf: *mut c_void, size: usize);

    fn sodium_misuse() -> !;
}

// =======================================================================
// crypto_pwhash/argon2/pwhash_argon2i.c
// =======================================================================

const CRYPTO_PWHASH_ARGON2I_ALG_ARGON2I13: c_int = 1;
const CRYPTO_PWHASH_ARGON2I_BYTES_MIN: usize = 16;
// SODIUM_MIN(SODIUM_SIZE_MAX, 4294967295U)
const CRYPTO_PWHASH_ARGON2I_BYTES_MAX: usize = {
    const A: u64 = SODIUM_SIZE_MAX as u64;
    const B: u64 = 4294967295u64;
    (if A < B { A } else { B }) as usize
};
const CRYPTO_PWHASH_ARGON2I_PASSWD_MIN: usize = 0;
const CRYPTO_PWHASH_ARGON2I_PASSWD_MAX: u64 = 4294967295;
const CRYPTO_PWHASH_ARGON2I_SALTBYTES: usize = 16;
const CRYPTO_PWHASH_ARGON2I_STRBYTES: usize = 128;
const CRYPTO_PWHASH_ARGON2I_OPSLIMIT_MIN: u64 = 3;
const CRYPTO_PWHASH_ARGON2I_OPSLIMIT_MAX: u64 = 4294967295;
const CRYPTO_PWHASH_ARGON2I_MEMLIMIT_MIN: usize = 8192;
// (SIZE_MAX >= 4398046510080) ? 4398046510080 : (SIZE_MAX >= 2147483648) ? 2147483648 : 32768
const CRYPTO_PWHASH_ARGON2I_MEMLIMIT_MAX: usize = {
    const SM: u64 = SODIUM_SIZE_MAX as u64;
    (if SM >= 4398046510080u64 {
        4398046510080u64
    } else if SM >= 2147483648u64 {
        2147483648u64
    } else {
        32768u64
    }) as usize
};

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_alg_argon2i13() -> c_int {
    1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_bytes_min() -> usize {
    CRYPTO_PWHASH_ARGON2I_BYTES_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_bytes_max() -> usize {
    CRYPTO_PWHASH_ARGON2I_BYTES_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_passwd_min() -> usize {
    CRYPTO_PWHASH_ARGON2I_PASSWD_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_passwd_max() -> usize {
    CRYPTO_PWHASH_ARGON2I_PASSWD_MAX as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_saltbytes() -> usize {
    CRYPTO_PWHASH_ARGON2I_SALTBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_strbytes() -> usize {
    CRYPTO_PWHASH_ARGON2I_STRBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_strprefix() -> *const c_char {
    b"$argon2i$\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_min() -> u64 {
    CRYPTO_PWHASH_ARGON2I_OPSLIMIT_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_max() -> u64 {
    CRYPTO_PWHASH_ARGON2I_OPSLIMIT_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_min() -> usize {
    CRYPTO_PWHASH_ARGON2I_MEMLIMIT_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_max() -> usize {
    CRYPTO_PWHASH_ARGON2I_MEMLIMIT_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_interactive() -> u64 {
    4
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_interactive() -> usize {
    33554432
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_moderate() -> u64 {
    6
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_moderate() -> usize {
    134217728
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_sensitive() -> u64 {
    8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_sensitive() -> usize {
    536870912
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i(
    out: *mut u8,
    outlen: u64,
    passwd: *const c_char,
    passwdlen: u64,
    salt: *const u8,
    opslimit: u64,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    memset(out as *mut c_void, 0, outlen as usize);
    if outlen > CRYPTO_PWHASH_ARGON2I_BYTES_MAX as u64 {
        set_errno(EFBIG);
        return -1;
    }
    if outlen < CRYPTO_PWHASH_ARGON2I_BYTES_MIN as u64 {
        set_errno(EINVAL);
        return -1;
    }
    if passwdlen > CRYPTO_PWHASH_ARGON2I_PASSWD_MAX
        || opslimit > CRYPTO_PWHASH_ARGON2I_OPSLIMIT_MAX
        || memlimit > CRYPTO_PWHASH_ARGON2I_MEMLIMIT_MAX
    {
        set_errno(EFBIG);
        return -1;
    }
    if opslimit < CRYPTO_PWHASH_ARGON2I_OPSLIMIT_MIN || memlimit < CRYPTO_PWHASH_ARGON2I_MEMLIMIT_MIN {
        set_errno(EINVAL);
        return -1;
    }
    if out as *const c_void == passwd as *const c_void {
        set_errno(EINVAL);
        return -1;
    }
    match alg {
        CRYPTO_PWHASH_ARGON2I_ALG_ARGON2I13 => {
            if argon2i_hash_raw(
                opslimit as u32,
                (memlimit / 1024) as u32,
                1u32,
                passwd as *const c_void,
                passwdlen as usize,
                salt as *const c_void,
                CRYPTO_PWHASH_ARGON2I_SALTBYTES,
                out as *mut c_void,
                outlen as usize,
            ) != ARGON2_OK
            {
                return -1;
            }
            0
        }
        _ => {
            set_errno(EINVAL);
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt: [u8; CRYPTO_PWHASH_ARGON2I_SALTBYTES] = [0u8; CRYPTO_PWHASH_ARGON2I_SALTBYTES];

    memset(out as *mut c_void, 0, CRYPTO_PWHASH_ARGON2I_STRBYTES);
    if passwdlen > CRYPTO_PWHASH_ARGON2I_PASSWD_MAX
        || opslimit > CRYPTO_PWHASH_ARGON2I_OPSLIMIT_MAX
        || memlimit > CRYPTO_PWHASH_ARGON2I_MEMLIMIT_MAX
    {
        set_errno(EFBIG);
        return -1;
    }
    if opslimit < CRYPTO_PWHASH_ARGON2I_OPSLIMIT_MIN || memlimit < CRYPTO_PWHASH_ARGON2I_MEMLIMIT_MIN {
        set_errno(EINVAL);
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, salt.len());
    if argon2i_hash_encoded(
        opslimit as u32,
        (memlimit / 1024) as u32,
        1u32,
        passwd as *const c_void,
        passwdlen as usize,
        salt.as_ptr() as *const c_void,
        salt.len(),
        32,
        out,
        CRYPTO_PWHASH_ARGON2I_STRBYTES,
    ) != ARGON2_OK
    {
        return -1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    if passwdlen > CRYPTO_PWHASH_ARGON2I_PASSWD_MAX {
        set_errno(EFBIG);
        return -1;
    }
    if passwdlen < CRYPTO_PWHASH_ARGON2I_PASSWD_MIN as u64 {
        set_errno(EINVAL);
        return -1;
    }

    let verify_ret = argon2i_verify(str_, passwd as *const c_void, passwdlen as usize);
    if verify_ret == ARGON2_OK {
        return 0;
    }
    if verify_ret == ARGON2_VERIFY_MISMATCH {
        set_errno(EINVAL);
    }
    -1
}

/// Shared by both crypto_pwhash_argon2i_str_needs_rehash and
/// crypto_pwhash_argon2id_str_needs_rehash (both defined, per the C source,
/// in pwhash_argon2i.c).
unsafe fn needs_rehash(str_: *const c_char, opslimit: u64, memlimit: usize, type_: argon2_type) -> c_int {
    let fodder_len: usize = strlen(str_);
    let memlimit = memlimit / 1024;
    if opslimit > u32::MAX as u64 || memlimit > u32::MAX as usize || fodder_len >= CRYPTO_PWHASH_ARGON2I_STRBYTES {
        set_errno(EINVAL);
        return -1;
    }
    let mut ctx: argon2_context = core::mem::zeroed();
    let fodder = calloc(fodder_len, 1) as *mut u8;
    if fodder.is_null() {
        return -1;
    }
    ctx.out = fodder;
    ctx.pwd = fodder;
    ctx.salt = fodder;
    ctx.outlen = fodder_len as u32;
    ctx.pwdlen = fodder_len as u32;
    ctx.saltlen = fodder_len as u32;
    ctx.ad = core::ptr::null_mut();
    ctx.secret = core::ptr::null_mut();
    ctx.adlen = 0;
    ctx.secretlen = 0;

    let ret;
    if argon2_decode_string(&mut ctx, str_, type_ as c_int) != 0 {
        set_errno(EINVAL);
        ret = -1;
    } else if ctx.t_cost != opslimit as u32 || ctx.m_cost != memlimit as u32 {
        ret = 1;
    } else {
        ret = 0;
    }
    free(fodder as *mut c_void);

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    needs_rehash(str_, opslimit, memlimit, argon2_type::Argon2_i)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    needs_rehash(str_, opslimit, memlimit, argon2_type::Argon2_id)
}

// =======================================================================
// crypto_pwhash/argon2/pwhash_argon2id.c
// =======================================================================

const CRYPTO_PWHASH_ARGON2ID_ALG_ARGON2ID13: c_int = 2;
const CRYPTO_PWHASH_ARGON2ID_BYTES_MIN: usize = 16;
const CRYPTO_PWHASH_ARGON2ID_BYTES_MAX: usize = CRYPTO_PWHASH_ARGON2I_BYTES_MAX;
const CRYPTO_PWHASH_ARGON2ID_PASSWD_MIN: usize = 0;
const CRYPTO_PWHASH_ARGON2ID_PASSWD_MAX: u64 = 4294967295;
const CRYPTO_PWHASH_ARGON2ID_SALTBYTES: usize = 16;
const CRYPTO_PWHASH_ARGON2ID_STRBYTES: usize = 128;
const CRYPTO_PWHASH_ARGON2ID_OPSLIMIT_MIN: u64 = 1;
const CRYPTO_PWHASH_ARGON2ID_OPSLIMIT_MAX: u64 = 4294967295;
const CRYPTO_PWHASH_ARGON2ID_MEMLIMIT_MIN: usize = 8192;
const CRYPTO_PWHASH_ARGON2ID_MEMLIMIT_MAX: usize = CRYPTO_PWHASH_ARGON2I_MEMLIMIT_MAX;

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_alg_argon2id13() -> c_int {
    2
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_bytes_min() -> usize {
    CRYPTO_PWHASH_ARGON2ID_BYTES_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_bytes_max() -> usize {
    CRYPTO_PWHASH_ARGON2ID_BYTES_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_passwd_min() -> usize {
    CRYPTO_PWHASH_ARGON2ID_PASSWD_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_passwd_max() -> usize {
    CRYPTO_PWHASH_ARGON2ID_PASSWD_MAX as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_saltbytes() -> usize {
    CRYPTO_PWHASH_ARGON2ID_SALTBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_strbytes() -> usize {
    CRYPTO_PWHASH_ARGON2ID_STRBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_strprefix() -> *const c_char {
    b"$argon2id$\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_min() -> u64 {
    CRYPTO_PWHASH_ARGON2ID_OPSLIMIT_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_max() -> u64 {
    CRYPTO_PWHASH_ARGON2ID_OPSLIMIT_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_min() -> usize {
    CRYPTO_PWHASH_ARGON2ID_MEMLIMIT_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_max() -> usize {
    CRYPTO_PWHASH_ARGON2ID_MEMLIMIT_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_interactive() -> u64 {
    2
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_interactive() -> usize {
    67108864
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_moderate() -> u64 {
    3
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_moderate() -> usize {
    268435456
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_sensitive() -> u64 {
    4
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_sensitive() -> usize {
    1073741824
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id(
    out: *mut u8,
    outlen: u64,
    passwd: *const c_char,
    passwdlen: u64,
    salt: *const u8,
    opslimit: u64,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    memset(out as *mut c_void, 0, outlen as usize);
    if outlen > CRYPTO_PWHASH_ARGON2ID_BYTES_MAX as u64 {
        set_errno(EFBIG);
        return -1;
    }
    if outlen < CRYPTO_PWHASH_ARGON2ID_BYTES_MIN as u64 {
        set_errno(EINVAL);
        return -1;
    }
    if passwdlen > CRYPTO_PWHASH_ARGON2ID_PASSWD_MAX
        || opslimit > CRYPTO_PWHASH_ARGON2ID_OPSLIMIT_MAX
        || memlimit > CRYPTO_PWHASH_ARGON2ID_MEMLIMIT_MAX
    {
        set_errno(EFBIG);
        return -1;
    }
    if opslimit < CRYPTO_PWHASH_ARGON2ID_OPSLIMIT_MIN || memlimit < CRYPTO_PWHASH_ARGON2ID_MEMLIMIT_MIN {
        set_errno(EINVAL);
        return -1;
    }
    if out as *const c_void == passwd as *const c_void {
        set_errno(EINVAL);
        return -1;
    }
    match alg {
        CRYPTO_PWHASH_ARGON2ID_ALG_ARGON2ID13 => {
            if argon2id_hash_raw(
                opslimit as u32,
                (memlimit / 1024) as u32,
                1u32,
                passwd as *const c_void,
                passwdlen as usize,
                salt as *const c_void,
                CRYPTO_PWHASH_ARGON2ID_SALTBYTES,
                out as *mut c_void,
                outlen as usize,
            ) != ARGON2_OK
            {
                return -1;
            }
            0
        }
        _ => {
            set_errno(EINVAL);
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt: [u8; CRYPTO_PWHASH_ARGON2ID_SALTBYTES] = [0u8; CRYPTO_PWHASH_ARGON2ID_SALTBYTES];

    memset(out as *mut c_void, 0, CRYPTO_PWHASH_ARGON2ID_STRBYTES);
    if passwdlen > CRYPTO_PWHASH_ARGON2ID_PASSWD_MAX
        || opslimit > CRYPTO_PWHASH_ARGON2ID_OPSLIMIT_MAX
        || memlimit > CRYPTO_PWHASH_ARGON2ID_MEMLIMIT_MAX
    {
        set_errno(EFBIG);
        return -1;
    }
    if opslimit < CRYPTO_PWHASH_ARGON2ID_OPSLIMIT_MIN || memlimit < CRYPTO_PWHASH_ARGON2ID_MEMLIMIT_MIN {
        set_errno(EINVAL);
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, salt.len());
    if argon2id_hash_encoded(
        opslimit as u32,
        (memlimit / 1024) as u32,
        1u32,
        passwd as *const c_void,
        passwdlen as usize,
        salt.as_ptr() as *const c_void,
        salt.len(),
        32,
        out,
        CRYPTO_PWHASH_ARGON2ID_STRBYTES,
    ) != ARGON2_OK
    {
        return -1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    if passwdlen > CRYPTO_PWHASH_ARGON2ID_PASSWD_MAX {
        set_errno(EFBIG);
        return -1;
    }
    if passwdlen < CRYPTO_PWHASH_ARGON2ID_PASSWD_MIN as u64 {
        set_errno(EINVAL);
        return -1;
    }

    let verify_ret = argon2id_verify(str_, passwd as *const c_void, passwdlen as usize);
    if verify_ret == ARGON2_OK {
        return 0;
    }
    if verify_ret == ARGON2_VERIFY_MISMATCH {
        set_errno(EINVAL);
    }
    -1
}

// =======================================================================
// crypto_pwhash/crypto_pwhash.c
// =======================================================================
//
// crypto_pwhash_* generic (algorithm-independent) definitions currently
// alias to the argon2id_* ones (crypto_pwhash_ALG_DEFAULT == ARGON2ID13).

const CRYPTO_PWHASH_ALG_ARGON2I13: c_int = 1;
const CRYPTO_PWHASH_ALG_ARGON2ID13: c_int = 2;

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_alg_argon2i13() -> c_int {
    1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_alg_argon2id13() -> c_int {
    2
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_alg_default() -> c_int {
    2
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_bytes_min() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_bytes_max() -> usize {
    CRYPTO_PWHASH_ARGON2ID_BYTES_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_passwd_min() -> usize {
    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_passwd_max() -> usize {
    4294967295
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_saltbytes() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_strbytes() -> usize {
    128
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_strprefix() -> *const c_char {
    b"$argon2id$\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_opslimit_min() -> u64 {
    1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_opslimit_max() -> u64 {
    4294967295
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_memlimit_min() -> usize {
    8192
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_memlimit_max() -> usize {
    CRYPTO_PWHASH_ARGON2ID_MEMLIMIT_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_opslimit_interactive() -> u64 {
    2
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_memlimit_interactive() -> usize {
    67108864
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_opslimit_moderate() -> u64 {
    3
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_memlimit_moderate() -> usize {
    268435456
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_opslimit_sensitive() -> u64 {
    4
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_memlimit_sensitive() -> usize {
    1073741824
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash(
    out: *mut u8,
    outlen: u64,
    passwd: *const c_char,
    passwdlen: u64,
    salt: *const u8,
    opslimit: u64,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    match alg {
        CRYPTO_PWHASH_ALG_ARGON2I13 => {
            crypto_pwhash_argon2i(out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg)
        }
        CRYPTO_PWHASH_ALG_ARGON2ID13 => {
            crypto_pwhash_argon2id(out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg)
        }
        _ => {
            set_errno(EINVAL);
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    crypto_pwhash_argon2id_str(out, passwd, passwdlen, opslimit, memlimit)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_str_alg(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    match alg {
        CRYPTO_PWHASH_ALG_ARGON2I13 => {
            return crypto_pwhash_argon2i_str(out, passwd, passwdlen, opslimit, memlimit);
        }
        CRYPTO_PWHASH_ALG_ARGON2ID13 => {
            return crypto_pwhash_argon2id_str(out, passwd, passwdlen, opslimit, memlimit);
        }
        _ => {}
    }
    sodium_misuse();
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    if strncmp(str_, b"$argon2id$\0".as_ptr() as *const c_char, 10) == 0 {
        return crypto_pwhash_argon2id_str_verify(str_, passwd, passwdlen);
    }
    if strncmp(str_, b"$argon2i$\0".as_ptr() as *const c_char, 9) == 0 {
        return crypto_pwhash_argon2i_str_verify(str_, passwd, passwdlen);
    }
    set_errno(EINVAL);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    if strncmp(str_, b"$argon2id$\0".as_ptr() as *const c_char, 10) == 0 {
        return crypto_pwhash_argon2id_str_needs_rehash(str_, opslimit, memlimit);
    }
    if strncmp(str_, b"$argon2i$\0".as_ptr() as *const c_char, 9) == 0 {
        return crypto_pwhash_argon2i_str_needs_rehash(str_, opslimit, memlimit);
    }
    set_errno(EINVAL);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn crypto_pwhash_primitive() -> *const c_char {
    b"argon2id,argon2i\0".as_ptr() as *const c_char
}
