//! Translation of c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c

use core::ffi::{c_char, c_int, c_void};

// EFBIG is not one of crate::plat's exported constants; on Linux x86_64 it is 27.
const EFBIG: c_int = 27;

// ---- constants (crypto_pwhash_argon2id.h) ----
const crypto_pwhash_argon2id_ALG_ARGON2ID13: c_int = 2;
const crypto_pwhash_argon2id_BYTES_MIN: usize = 16;
const crypto_pwhash_argon2id_BYTES_MAX: usize = crate::common::SODIUM_SIZE_MAX & 4294967295;
const crypto_pwhash_argon2id_PASSWD_MIN: u64 = 0;
const crypto_pwhash_argon2id_PASSWD_MAX: u64 = 4294967295;
const crypto_pwhash_argon2id_SALTBYTES: usize = 16;
const crypto_pwhash_argon2id_STRBYTES: usize = 128;
const crypto_pwhash_argon2id_STRPREFIX: &[u8] = b"$argon2id$\0";
const crypto_pwhash_argon2id_OPSLIMIT_MIN: u64 = 1;
const crypto_pwhash_argon2id_OPSLIMIT_MAX: u64 = 4294967295;
const crypto_pwhash_argon2id_MEMLIMIT_MIN: usize = 8192;
const crypto_pwhash_argon2id_MEMLIMIT_MAX: usize = 4398046510080;
const crypto_pwhash_argon2id_OPSLIMIT_INTERACTIVE: u64 = 2;
const crypto_pwhash_argon2id_MEMLIMIT_INTERACTIVE: usize = 67108864;
const crypto_pwhash_argon2id_OPSLIMIT_MODERATE: u64 = 3;
const crypto_pwhash_argon2id_MEMLIMIT_MODERATE: usize = 268435456;
const crypto_pwhash_argon2id_OPSLIMIT_SENSITIVE: u64 = 4;
const crypto_pwhash_argon2id_MEMLIMIT_SENSITIVE: usize = 1073741824;

const STR_HASHBYTES: usize = 32;

const ARGON2_OK: c_int = 0;
const ARGON2_VERIFY_MISMATCH: c_int = -35;

extern "C" {
    fn _sodium_argon2id_hash_raw(
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
    fn _sodium_argon2id_hash_encoded(
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
    fn _sodium_argon2id_verify(
        encoded: *const c_char,
        pwd: *const c_void,
        pwdlen: usize,
    ) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_alg_argon2id13() -> c_int {
    crypto_pwhash_argon2id_ALG_ARGON2ID13
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_bytes_min() -> usize {
    crypto_pwhash_argon2id_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_bytes_max() -> usize {
    crypto_pwhash_argon2id_BYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_passwd_min() -> usize {
    crypto_pwhash_argon2id_PASSWD_MIN as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_passwd_max() -> usize {
    crypto_pwhash_argon2id_PASSWD_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_saltbytes() -> usize {
    crypto_pwhash_argon2id_SALTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_strbytes() -> usize {
    crypto_pwhash_argon2id_STRBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_strprefix() -> *const c_char {
    crypto_pwhash_argon2id_STRPREFIX.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_min() -> u64 {
    crypto_pwhash_argon2id_OPSLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_max() -> u64 {
    crypto_pwhash_argon2id_OPSLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_min() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_max() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_interactive() -> u64 {
    crypto_pwhash_argon2id_OPSLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_interactive() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_INTERACTIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_moderate() -> u64 {
    crypto_pwhash_argon2id_OPSLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_moderate() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_MODERATE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_opslimit_sensitive() -> u64 {
    crypto_pwhash_argon2id_OPSLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_memlimit_sensitive() -> usize {
    crypto_pwhash_argon2id_MEMLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
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
    if outlen as usize > crypto_pwhash_argon2id_BYTES_MAX {
        crate::plat::set_errno(EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    if (outlen as usize) < crypto_pwhash_argon2id_BYTES_MIN {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    if passwdlen > crypto_pwhash_argon2id_PASSWD_MAX
        || opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX
    {
        crate::plat::set_errno(EFBIG);
        return -1;
    }
    if passwdlen < crypto_pwhash_argon2id_PASSWD_MIN
        || opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN
    {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    if out as *const c_void == passwd as *const c_void {
        crate::plat::set_errno(crate::plat::EINVAL); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    match alg {
        x if x == crypto_pwhash_argon2id_ALG_ARGON2ID13 => {
            if _sodium_argon2id_hash_raw(
                opslimit as u32,
                (memlimit / 1024) as u32,
                1u32,
                passwd as *const c_void,
                passwdlen as usize,
                salt as *const c_void,
                crypto_pwhash_argon2id_SALTBYTES,
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
pub unsafe extern "C" fn crypto_pwhash_argon2id_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    let mut salt: [u8; crypto_pwhash_argon2id_SALTBYTES] = [0; crypto_pwhash_argon2id_SALTBYTES];

    memset(out as *mut c_void, 0, crypto_pwhash_argon2id_STRBYTES);
    if passwdlen > crypto_pwhash_argon2id_PASSWD_MAX
        || opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX
        || memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX
    {
        crate::plat::set_errno(EFBIG);
        return -1;
    }
    if passwdlen < crypto_pwhash_argon2id_PASSWD_MIN
        || opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN
        || memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN
    {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    randombytes_buf(salt.as_mut_ptr() as *mut c_void, salt.len());
    if _sodium_argon2id_hash_encoded(
        opslimit as u32,
        (memlimit / 1024) as u32,
        1u32,
        passwd as *const c_void,
        passwdlen as usize,
        salt.as_ptr() as *const c_void,
        salt.len(),
        STR_HASHBYTES,
        out,
        crypto_pwhash_argon2id_STRBYTES,
    ) != ARGON2_OK
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_argon2id_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    let verify_ret: c_int;

    if passwdlen > crypto_pwhash_argon2id_PASSWD_MAX {
        crate::plat::set_errno(EFBIG); /* LCOV_EXCL_LINE */
        return -1; /* LCOV_EXCL_LINE */
    }
    /* LCOV_EXCL_START */
    if passwdlen < crypto_pwhash_argon2id_PASSWD_MIN {
        crate::plat::set_errno(crate::plat::EINVAL);
        return -1;
    }
    /* LCOV_EXCL_STOP */

    verify_ret = _sodium_argon2id_verify(str_, passwd as *const c_void, passwdlen as usize);
    if verify_ret == ARGON2_OK {
        return 0;
    }
    if verify_ret == ARGON2_VERIFY_MISMATCH {
        crate::plat::set_errno(crate::plat::EINVAL);
    }
    -1
}
