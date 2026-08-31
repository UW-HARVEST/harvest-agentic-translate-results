//! Translation of c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c

use core::ffi::{c_char, c_int, c_ulong};

const ARGON2_OK: c_int = 0;
const ARGON2_DECODING_FAIL: c_int = -32;
const ARGON2_INCORRECT_TYPE: c_int = -26;
const ARGON2_ENCODING_FAIL: c_int = -31;

const ARGON2_VERSION_NUMBER: u32 = 0x13;

const Argon2_i: c_int = 1;
const Argon2_id: c_int = 2;

// sodium_base64_VARIANT_ORIGINAL_NO_PADDING (utils.h)
const sodium_base64_VARIANT_ORIGINAL_NO_PADDING: c_int = 3;

const U32_STR_MAXSIZE: usize = 11;

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
    // argon2-core.c -> _sodium_argon2_validate_inputs
    fn _sodium_argon2_validate_inputs(context: *const argon2_context) -> c_int;
    // exported utils
    fn sodium_bin2base64(
        b64: *mut c_char,
        b64_maxlen: usize,
        bin: *const u8,
        bin_len: usize,
        variant: c_int,
    ) -> *mut c_char;
    fn sodium_base642bin(
        bin: *mut u8,
        bin_maxlen: usize,
        b64: *const c_char,
        b64_len: usize,
        ignore: *const c_char,
        bin_len: *mut usize,
        b64_end: *mut *const c_char,
        variant: c_int,
    ) -> c_int;
    // libc
    fn strlen(s: *const c_char) -> usize;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
}

/*
 * Decode decimal integer from 'str'; the value is written in '*v'.
 */
unsafe fn decode_decimal(mut str_: *const c_char, v: *mut c_ulong) -> *const c_char {
    let orig: *const c_char;
    let mut acc: c_ulong;

    acc = 0;
    orig = str_;
    loop {
        let mut c: c_int;

        c = *str_ as c_int;
        if c < b'0' as c_int || c > b'9' as c_int {
            break;
        }
        c -= b'0' as c_int;
        if acc > (c_ulong::MAX / 10) {
            return core::ptr::null(); /* LCOV_EXCL_LINE */
        }
        acc = acc.wrapping_mul(10);
        if (c as c_ulong) > (c_ulong::MAX - acc) {
            return core::ptr::null(); /* LCOV_EXCL_LINE */
        }
        acc = acc.wrapping_add(c as c_ulong);

        str_ = str_.add(1);
    }
    if str_ == orig || (*orig == b'0' as c_char && str_ != orig.add(1)) {
        return core::ptr::null(); /* LCOV_EXCL_LINE */
    }
    *v = acc;
    str_
}

// argon2_decode_string -> _sodium_argon2_decode_string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_decode_string(
    ctx: *mut argon2_context,
    mut str_: *const c_char,
    type_: c_int,
) -> c_int {
    // CC(prefix): check literal prefix, advance str; return DECODING_FAIL on mismatch.
    macro_rules! cc {
        ($prefix:expr) => {{
            let prefix: &[u8] = $prefix; // NUL-terminated byte literal
            let cc_len = strlen(prefix.as_ptr() as *const c_char);
            if strncmp(str_, prefix.as_ptr() as *const c_char, cc_len) != 0 {
                return ARGON2_DECODING_FAIL;
            }
            str_ = str_.add(cc_len);
        }};
    }

    // DECIMAL_U32(x): decode decimal, fail if NULL or > UINT32_MAX.
    macro_rules! decimal_u32 {
        ($x:expr) => {{
            let mut dec_x: c_ulong = 0;
            str_ = decode_decimal(str_, &mut dec_x);
            if str_.is_null() || dec_x > u32::MAX as c_ulong {
                return ARGON2_DECODING_FAIL;
            }
            $x = dec_x as u32;
        }};
    }

    // BIN(buf, max_len, len): base64 decode.
    macro_rules! bin {
        ($buf:expr, $max_len:expr, $len:expr) => {{
            let mut bin_len: usize = $max_len;
            let mut str_end: *const c_char = core::ptr::null();
            if sodium_base642bin(
                $buf,
                $max_len,
                str_,
                strlen(str_),
                core::ptr::null(),
                &mut bin_len,
                &mut str_end,
                sodium_base64_VARIANT_ORIGINAL_NO_PADDING,
            ) != 0
                || bin_len > u32::MAX as usize
            {
                return ARGON2_DECODING_FAIL;
            }
            $len = bin_len as u32;
            str_ = str_end;
        }};
    }

    let maxsaltlen: usize = (*ctx).saltlen as usize;
    let maxoutlen: usize = (*ctx).outlen as usize;
    let validation_result: c_int;
    let mut version: u32 = 0;

    (*ctx).saltlen = 0;
    (*ctx).outlen = 0;

    if type_ == Argon2_id {
        cc!(b"$argon2id\0");
    } else if type_ == Argon2_i {
        cc!(b"$argon2i\0");
    } else {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    cc!(b"$v=\0");
    decimal_u32!(version);
    if version != ARGON2_VERSION_NUMBER {
        return ARGON2_INCORRECT_TYPE;
    }
    cc!(b"$m=\0");
    decimal_u32!((*ctx).m_cost);
    if (*ctx).m_cost > u32::MAX {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    cc!(b",t=\0");
    decimal_u32!((*ctx).t_cost);
    if (*ctx).t_cost > u32::MAX {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    cc!(b",p=\0");
    decimal_u32!((*ctx).lanes);
    if (*ctx).lanes > u32::MAX {
        return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */
    }
    (*ctx).threads = (*ctx).lanes;

    cc!(b"$\0");
    bin!((*ctx).salt, maxsaltlen, (*ctx).saltlen);
    cc!(b"$\0");
    bin!((*ctx).out, maxoutlen, (*ctx).outlen);
    validation_result = _sodium_argon2_validate_inputs(ctx);
    if validation_result != ARGON2_OK {
        return validation_result;
    }
    if *str_ == 0 {
        return ARGON2_OK;
    }
    ARGON2_DECODING_FAIL
}

unsafe fn u32_to_string(str_: *mut c_char, mut x: u32) {
    let mut tmp: [c_char; U32_STR_MAXSIZE - 1] = [0; U32_STR_MAXSIZE - 1];
    let mut i: usize;

    i = core::mem::size_of_val(&tmp); // sizeof tmp == 10
    loop {
        i -= 1;
        tmp[i] = ((x % 10u32) as u8 + b'0') as c_char;
        x /= 10u32;
        if !(x != 0 && i != 0) {
            break;
        }
    }
    core::ptr::copy_nonoverlapping(
        tmp.as_ptr().add(i),
        str_,
        core::mem::size_of_val(&tmp) - i,
    );
    *str_.add(core::mem::size_of_val(&tmp) - i) = 0;
}

// argon2_encode_string -> _sodium_argon2_encode_string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_argon2_encode_string(
    mut dst: *mut c_char,
    mut dst_len: usize,
    ctx: *mut argon2_context,
    type_: c_int,
) -> c_int {
    // SS(str): append a NUL-terminated string (incl. its NUL), advance dst.
    macro_rules! ss {
        ($lit:expr) => {{
            let src: *const c_char = $lit;
            let pp_len = strlen(src);
            if pp_len >= dst_len {
                return ARGON2_ENCODING_FAIL;
            }
            core::ptr::copy_nonoverlapping(src, dst, pp_len + 1);
            dst = dst.add(pp_len);
            dst_len -= pp_len;
        }};
    }

    // SX(x): format u32 then SS it.
    macro_rules! sx {
        ($x:expr) => {{
            let mut tmp: [c_char; U32_STR_MAXSIZE] = [0; U32_STR_MAXSIZE];
            u32_to_string(tmp.as_mut_ptr(), $x);
            ss!(tmp.as_ptr());
        }};
    }

    // SB(buf, len): base64 encode into dst.
    macro_rules! sb {
        ($buf:expr, $len:expr) => {{
            let sb_len: usize;
            if sodium_bin2base64(
                dst,
                dst_len,
                $buf,
                $len,
                sodium_base64_VARIANT_ORIGINAL_NO_PADDING,
            )
            .is_null()
            {
                return ARGON2_ENCODING_FAIL;
            }
            sb_len = strlen(dst);
            dst = dst.add(sb_len);
            dst_len -= sb_len;
        }};
    }

    let validation_result: c_int;

    match type_ {
        x if x == Argon2_id => {
            ss!(b"$argon2id$v=\0".as_ptr() as *const c_char);
        }
        x if x == Argon2_i => {
            ss!(b"$argon2i$v=\0".as_ptr() as *const c_char);
        }
        _ => {
            return ARGON2_ENCODING_FAIL; /* LCOV_EXCL_LINE */
        }
    }
    validation_result = _sodium_argon2_validate_inputs(ctx);
    if validation_result != ARGON2_OK {
        return validation_result; /* LCOV_EXCL_LINE */
    }
    sx!(ARGON2_VERSION_NUMBER);
    ss!(b"$m=\0".as_ptr() as *const c_char);
    sx!((*ctx).m_cost);
    ss!(b",t=\0".as_ptr() as *const c_char);
    sx!((*ctx).t_cost);
    ss!(b",p=\0".as_ptr() as *const c_char);
    sx!((*ctx).lanes);

    ss!(b"$\0".as_ptr() as *const c_char);
    sb!((*ctx).salt, (*ctx).saltlen as usize);

    ss!(b"$\0".as_ptr() as *const c_char);
    sb!((*ctx).out, (*ctx).outlen as usize);
    ARGON2_OK
}
