//! Translation of c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c

use core::ffi::{c_char, c_int, c_void};

// EFBIG is not one of crate::plat's exported constants; on Linux x86_64 it is 27.
const EFBIG: c_int = 27;

// ---- constants (crypto_pwhash_argon2i.h) ----
const crypto_pwhash_argon2i_ALG_ARGON2I13: c_int = 1;
const crypto_pwhash_argon2i_BYTES_MIN: usize = 16;
const crypto_pwhash_argon2i_BYTES_MAX: usize = crate::common::SODIUM_SIZE_MAX & 4294967295;
const crypto_pwhash_argon2i_PASSWD_MIN: u64 = 0;
const crypto_pwhash_argon2i_PASSWD_MAX: u64 = 4294967295;
const crypto_pwhash_argon2i_SALTBYTES: usize = 16;
const crypto_pwhash_argon2i_STRBYTES: usize = 128;
const crypto_pwhash_argon2i_STRPREFIX: &[u8] = b"$argon2i$\0";
const crypto_pwhash_argon2i_OPSLIMIT_MIN: u64 = 3;
const crypto_pwhash_argon2i_OPSLIMIT_MAX: u64 = 4294967295;
const crypto_pwhash_argon2i_MEMLIMIT_MIN: usize = 8192;
const crypto_pwhash_argon2i_MEMLIMIT_MAX: usize = 4398046510080;
const crypto_pwhash_argon2i_OPSLIMIT_INTERACTIVE: u64 = 4;
const crypto_pwhash_argon2i_MEMLIMIT_INTERACTIVE: usize = 33554432;
const crypto_pwhash_argon2i_OPSLIMIT_MODERATE: u64 = 6;
const crypto_pwhash_argon2i_MEMLIMIT_MODERATE: usize = 134217728;
const crypto_pwhash_argon2i_OPSLIMIT_SENSITIVE: u64 = 8;
const crypto_pwhash_argon2i_MEMLIMIT_SENSITIVE: usize = 536870912;

const crypto_pwhash_STRBYTES: usize = 128;

const STR_HASHBYTES: usize = 32;

const ARGON2_OK: c_int = 0;
const ARGON2_VERIFY_MISMATCH: c_int = -35;

const Argon2_i: c_int = 1;
const Argon2_id: c_int = 2;

// argon2_context #[repr(C)] (argon2.h)
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
    fn _sodium_argon2i_hash_raw(
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
    fn _sodium_argon2i_hash_encoded(
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
    fn _sodium_argon2i_verify(
        encoded: *const c_char,
        pwd: *const c_void,
        pwdlen: usize,
    ) -> c_int;
    fn _sodium_argon2_decode_string(
        ctx: *mut argon2_context,
        str_: *const c_char,
        type_: c_int,
    ) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_alg_argon2i13() -> c_int {
    crypto_pwhash_argon2i_ALG_ARGON2I13
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_bytes_min() -> usize {
    crypto_pwhash_argon2i_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_bytes_max() -> usize {
    crypto_pwhash_argon2i_BYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_passwd_min() -> usize {
    crypto_pwhash_argon2i_PASSWD_MIN as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_passwd_max() -> usize {
    crypto_pwhash_argon2i_PASSWD_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_saltbytes() -> usize {
    crypto_pwhash_argon2i_SALTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_strbytes() -> usize {
    crypto_pwhash_argon2i_STRBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_strprefix() -> *const c_char {
    crypto_pwhash_argon2i_STRPREFIX.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_min() -> u64 {
    crypto_pwhash_argon2i_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_max() -> u64 {
    crypto_pwhash_argon2i_OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_min() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_max() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_interactive() -> u64 {
    crypto_pwhash_argon2i_OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_interactive() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_moderate() -> u64 {
    crypto_pwhash_argon2i_OPSLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_moderate() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_opslimit_sensitive() -> u64 {
    crypto_pwhash_argon2i_OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_memlimit_sensitive() -> usize {
    crypto_pwhash_argon2i_MEMLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
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
    if outlen as usize > crypto_pwhash_argon2i_BYTES_MAX {
        crate::plat::set_errno(EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if (outlen as usize) < crypto_pwhash_argon2i_BYTES_MIN {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX
        || opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX
    {
        crate::plat::set_errno(EFBIG);
        return -1;
    }
    if passwdlen < crypto_pwhash_argon2i_PASSWD_MIN
        || opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN
    {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    if out as *const c_void == passwd as *const c_void {
        crate::plat::set_errno(crate::plat::EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    match alg {
        x if x == crypto_pwhash_argon2i_ALG_ARGON2I13 => {
            if _sodium_argon2i_hash_raw(
                opslimit as u32,
                (memlimit / 1024) as u32,
                1u32,
                passwd as *const c_void,
                passwdlen as usize,
                salt as *const c_void,
                crypto_pwhash_argon2i_SALTBYTES,
                out as *mut c_void,
                outlen as usize,
            ) != ARGON2_OK
            {
                return -1; /* LCOV_EXCL_LINE */
            }
            0
        }
        _ => {
            crate::plat::set_errno(crate::plat::EINVAL);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt: [u8; crypto_pwhash_argon2i_SALTBYTES] = [0; crypto_pwhash_argon2i_SALTBYTES];

    memset(out as *mut c_void, 0, crypto_pwhash_argon2i_STRBYTES);
    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX
        || opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX
    {
        crate::plat::set_errno(EFBIG);
        return -1;
    }
    if passwdlen < crypto_pwhash_argon2i_PASSWD_MIN
        || opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN
    {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, salt.len());
    if _sodium_argon2i_hash_encoded(
        opslimit as u32,
        (memlimit / 1024) as u32,
        1u32,
        passwd as *const c_void,
        passwdlen as usize,
        salt.as_ptr() as *const c_void,
        salt.len(),
        STR_HASHBYTES,
        out,
        crypto_pwhash_argon2i_STRBYTES,
    ) != ARGON2_OK
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    let verify_ret: c_int;

    if passwdlen > crypto_pwhash_argon2i_PASSWD_MAX {
        crate::plat::set_errno(EFBIG);
        return -1;
    }
    /* LCOV_EXCL_START */
    if passwdlen < crypto_pwhash_argon2i_PASSWD_MIN {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    /* LCOV_EXCL_STOP */

    verify_ret = _sodium_argon2i_verify(str_, passwd as *const c_void, passwdlen as usize);
    if verify_ret == ARGON2_OK {
        return 0;
    }
    if verify_ret == ARGON2_VERIFY_MISMATCH {
        crate::plat::set_errno(crate::plat::EINVAL);
    }
    -1
}

unsafe fn _needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    mut memlimit: usize,
    type_: c_int,
) -> c_int {
    let fodder: *mut u8;
    let mut ctx: argon2_context = core::mem::zeroed();
    let fodder_len: usize;
    let ret: c_int;

    fodder_len = strlen(str_);
    memlimit /= 1024;
    if opslimit > u32::MAX as u64
        || memlimit > u32::MAX as usize
        || fodder_len >= crypto_pwhash_STRBYTES
    {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    memset(
        &mut ctx as *mut argon2_context as *mut c_void,
        0,
        core::mem::size_of::<argon2_context>(),
    );
    fodder = calloc(fodder_len, 1) as *mut u8;
    if fodder.is_null() {
        return -1; /* LCOV_EXCL_LINE */
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
    if _sodium_argon2_decode_string(&mut ctx, str_, type_) != 0 {
        crate::plat::set_errno(crate::plat::EINVAL);
        ret = -1;
    } else if ctx.t_cost != opslimit as u32 || ctx.m_cost != memlimit as u32 {
        ret = 1;
    } else {
        ret = 0;
    }
    free(fodder as *mut c_void);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2i_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    _needs_rehash(str_, opslimit, memlimit, Argon2_i)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    _needs_rehash(str_, opslimit, memlimit, Argon2_id)
}
