use super::sign;
use crate::params::*;
use std::ffi::{c_int, c_uchar, c_ulonglong};

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> c_ulonglong { SK_BYTES as c_ulonglong }
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> c_ulonglong { PK_BYTES as c_ulonglong }
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> c_ulonglong { BYTES as c_ulonglong }
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> c_ulonglong { SEED_BYTES as c_ulonglong }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut c_uchar, sk: *mut c_uchar, seed: *const c_uchar
) -> c_int {
    sign::crypto_sign_seed_keypair(
        unsafe { std::slice::from_raw_parts_mut(pk, PK_BYTES) },
        unsafe { std::slice::from_raw_parts_mut(sk, SK_BYTES) },
        unsafe { std::slice::from_raw_parts(seed, SEED_BYTES) },
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut c_uchar, sk: *mut c_uchar) -> c_int {
    sign::crypto_sign_keypair(
        unsafe { std::slice::from_raw_parts_mut(pk, PK_BYTES) },
        unsafe { std::slice::from_raw_parts_mut(sk, SK_BYTES) },
        None,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize, m: *const u8, mlen: usize, sk: *const u8
) -> c_int {
    sign::crypto_sign_signature(
        unsafe { std::slice::from_raw_parts_mut(sig, BYTES) },
        unsafe { std::slice::from_raw_parts(m, mlen) },
        unsafe { std::slice::from_raw_parts(sk, SK_BYTES) },
        None,
    );
    unsafe { *siglen = BYTES };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize, m: *const u8, mlen: usize, pk: *const u8
) -> c_int {
    if siglen != BYTES { return -1; }
    sign::crypto_sign_verify(
        unsafe { std::slice::from_raw_parts(sig, siglen) },
        unsafe { std::slice::from_raw_parts(m, mlen) },
        unsafe { std::slice::from_raw_parts(pk, PK_BYTES) },
    ).map_or(-1, |_| 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut c_ulonglong, m: *const u8, mlen: c_ulonglong, sk: *const u8
) -> c_int {
    let mlen = mlen as usize;
    let msg = unsafe { std::slice::from_raw_parts(m, mlen) }.to_vec();
    let out = unsafe { std::slice::from_raw_parts_mut(sm, BYTES + mlen) };
    sign::crypto_sign_signature(&mut out[..BYTES], &msg, unsafe { std::slice::from_raw_parts(sk, SK_BYTES) }, None);
    out[BYTES..].copy_from_slice(&msg);
    unsafe { *smlen = (BYTES + mlen) as u64 };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut c_ulonglong, sm: *const u8, smlen: c_ulonglong, pk: *const u8
) -> c_int {
    let smlen = smlen as usize;
    if smlen < BYTES {
        unsafe { std::ptr::write_bytes(m, 0, smlen); *mlen = 0; }
        return -1;
    }
    let signed = unsafe { std::slice::from_raw_parts(sm, smlen) };
    let msg = &signed[BYTES..];
    if sign::crypto_sign_verify(&signed[..BYTES], msg, unsafe { std::slice::from_raw_parts(pk, PK_BYTES) }).is_err() {
        unsafe { std::ptr::write_bytes(m, 0, smlen); *mlen = 0; }
        return -1;
    }
    unsafe { std::ptr::copy(msg.as_ptr(), m, msg.len()); *mlen = msg.len() as u64; }
    0
}
