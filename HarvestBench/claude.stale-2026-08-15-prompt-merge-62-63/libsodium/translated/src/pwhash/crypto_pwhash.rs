//! Translation of crypto_pwhash/crypto_pwhash.c (top-level dispatch).

use core::ffi::{c_char, c_int};

extern "C" {
    fn crypto_pwhash_argon2i(
        out: *mut u8,
        outlen: u64,
        passwd: *const c_char,
        passwdlen: u64,
        salt: *const u8,
        opslimit: u64,
        memlimit: usize,
        alg: c_int,
    ) -> c_int;
    fn crypto_pwhash_argon2id(
        out: *mut u8,
        outlen: u64,
        passwd: *const c_char,
        passwdlen: u64,
        salt: *const u8,
        opslimit: u64,
        memlimit: usize,
        alg: c_int,
    ) -> c_int;
    fn crypto_pwhash_argon2i_str(
        out: *mut c_char,
        passwd: *const c_char,
        passwdlen: u64,
        opslimit: u64,
        memlimit: usize,
    ) -> c_int;
    fn crypto_pwhash_argon2id_str(
        out: *mut c_char,
        passwd: *const c_char,
        passwdlen: u64,
        opslimit: u64,
        memlimit: usize,
    ) -> c_int;
    fn crypto_pwhash_argon2i_str_verify(
        str_: *const c_char,
        passwd: *const c_char,
        passwdlen: u64,
    ) -> c_int;
    fn crypto_pwhash_argon2id_str_verify(
        str_: *const c_char,
        passwd: *const c_char,
        passwdlen: u64,
    ) -> c_int;
    fn crypto_pwhash_argon2i_str_needs_rehash(
        str_: *const c_char,
        opslimit: u64,
        memlimit: usize,
    ) -> c_int;
    fn crypto_pwhash_argon2id_str_needs_rehash(
        str_: *const c_char,
        opslimit: u64,
        memlimit: usize,
    ) -> c_int;
    fn sodium_misuse() -> !;
}

#[inline]
unsafe fn set_errno(e: c_int) {
    *libc::__errno_location() = e;
}

// crypto_pwhash_* == crypto_pwhash_argon2id_* aliases.
const ALG_ARGON2I13: c_int = 1;
const ALG_ARGON2ID13: c_int = 2;
const ALG_DEFAULT: c_int = ALG_ARGON2ID13;

const BYTES_MIN: usize = 16;
const BYTES_MAX: usize = 4294967295;
const PASSWD_MIN: usize = 0;
const PASSWD_MAX: usize = 4294967295;
const SALTBYTES: usize = 16;
const STRBYTES: usize = 128;
const STRPREFIX: &[u8] = b"$argon2id$\0";
const OPSLIMIT_MIN: u64 = 1;
const OPSLIMIT_MAX: u64 = 4294967295;
const MEMLIMIT_MIN: usize = 8192;
const MEMLIMIT_MAX: usize = 4398046510080;
const OPSLIMIT_INTERACTIVE: u64 = 2;
const MEMLIMIT_INTERACTIVE: usize = 67108864;
const OPSLIMIT_MODERATE: u64 = 3;
const MEMLIMIT_MODERATE: usize = 268435456;
const OPSLIMIT_SENSITIVE: u64 = 4;
const MEMLIMIT_SENSITIVE: usize = 1073741824;

const PRIMITIVE: &[u8] = b"argon2id,argon2i\0";

const ARGON2ID_STRPREFIX: &[u8] = b"$argon2id$";
const ARGON2I_STRPREFIX: &[u8] = b"$argon2i$";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_alg_argon2i13() -> c_int {
    ALG_ARGON2I13
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_alg_argon2id13() -> c_int {
    ALG_ARGON2ID13
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_alg_default() -> c_int {
    ALG_DEFAULT
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_bytes_min() -> usize {
    BYTES_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_bytes_max() -> usize {
    BYTES_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_passwd_min() -> usize {
    PASSWD_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_passwd_max() -> usize {
    PASSWD_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_saltbytes() -> usize {
    SALTBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_strbytes() -> usize {
    STRBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_strprefix() -> *const c_char {
    STRPREFIX.as_ptr() as *const c_char
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_opslimit_min() -> u64 {
    OPSLIMIT_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_opslimit_max() -> u64 {
    OPSLIMIT_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_memlimit_min() -> usize {
    MEMLIMIT_MIN
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_memlimit_max() -> usize {
    MEMLIMIT_MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_opslimit_interactive() -> u64 {
    OPSLIMIT_INTERACTIVE
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_memlimit_interactive() -> usize {
    MEMLIMIT_INTERACTIVE
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_opslimit_moderate() -> u64 {
    OPSLIMIT_MODERATE
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_memlimit_moderate() -> usize {
    MEMLIMIT_MODERATE
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_opslimit_sensitive() -> u64 {
    OPSLIMIT_SENSITIVE
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_memlimit_sensitive() -> usize {
    MEMLIMIT_SENSITIVE
}

#[unsafe(no_mangle)]
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
        ALG_ARGON2I13 => {
            crypto_pwhash_argon2i(out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg)
        }
        ALG_ARGON2ID13 => {
            crypto_pwhash_argon2id(out, outlen, passwd, passwdlen, salt, opslimit, memlimit, alg)
        }
        _ => {
            set_errno(libc::EINVAL);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    crypto_pwhash_argon2id_str(out, passwd, passwdlen, opslimit, memlimit)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_alg(
    out: *mut c_char,
    passwd: *const c_char,
    passwdlen: u64,
    opslimit: u64,
    memlimit: usize,
    alg: c_int,
) -> c_int {
    match alg {
        ALG_ARGON2I13 => crypto_pwhash_argon2i_str(out, passwd, passwdlen, opslimit, memlimit),
        ALG_ARGON2ID13 => crypto_pwhash_argon2id_str(out, passwd, passwdlen, opslimit, memlimit),
        _ => {
            sodium_misuse();
        }
    }
}

/// strncmp against a prefix (without the trailing NUL), C `strncmp(str, prefix, n) == 0`.
unsafe fn prefix_matches(str_: *const c_char, prefix: &[u8]) -> bool {
    let s = str_ as *const u8;
    for (i, &b) in prefix.iter().enumerate() {
        let c = *s.add(i);
        if c != b {
            return false;
        }
        // If prefix byte and str byte are equal but nonzero we continue.
        // strncmp stops at n bytes; since prefix has no interior NUL, we just
        // compare n bytes. If str is shorter (has a NUL), c != b already caught it
        // unless b is 0, but prefix bytes are all nonzero here.
        let _ = c;
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_verify(
    str_: *const c_char,
    passwd: *const c_char,
    passwdlen: u64,
) -> c_int {
    if prefix_matches(str_, ARGON2ID_STRPREFIX) {
        return crypto_pwhash_argon2id_str_verify(str_, passwd, passwdlen);
    }
    if prefix_matches(str_, ARGON2I_STRPREFIX) {
        return crypto_pwhash_argon2i_str_verify(str_, passwd, passwdlen);
    }
    set_errno(libc::EINVAL);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_pwhash_str_needs_rehash(
    str_: *const c_char,
    opslimit: u64,
    memlimit: usize,
) -> c_int {
    if prefix_matches(str_, ARGON2ID_STRPREFIX) {
        return crypto_pwhash_argon2id_str_needs_rehash(str_, opslimit, memlimit);
    }
    if prefix_matches(str_, ARGON2I_STRPREFIX) {
        return crypto_pwhash_argon2i_str_needs_rehash(str_, opslimit, memlimit);
    }
    set_errno(libc::EINVAL);
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_pwhash_primitive() -> *const c_char {
    PRIMITIVE.as_ptr() as *const c_char
}
