//! Translation of c_src/libsodium/crypto_stream/salsa20/stream_salsa20.c

use core::ffi::{c_int, c_void};

// crypto_stream_salsa20_* constants
const CRYPTO_STREAM_SALSA20_KEYBYTES: usize = 32;
const CRYPTO_STREAM_SALSA20_NONCEBYTES: usize = 8;
// crypto_stream_salsa20_MESSAGEBYTES_MAX = SODIUM_SIZE_MAX = min(UINT64_MAX, SIZE_MAX)
const CRYPTO_STREAM_SALSA20_MESSAGEBYTES_MAX: usize = usize::MAX;

// #[repr(C)] mirror of `crypto_stream_salsa20_implementation` from
// crypto_stream/salsa20/stream_salsa20.h.
#[repr(C)]
pub struct CryptoStreamSalsa20Implementation {
    pub stream: Option<
        unsafe extern "C" fn(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int,
    >,
    pub stream_xor_ic: Option<
        unsafe extern "C" fn(
            c: *mut u8,
            m: *const u8,
            mlen: u64,
            n: *const u8,
            ic: u64,
            k: *const u8,
        ) -> c_int,
    >,
}

unsafe impl Sync for CryptoStreamSalsa20Implementation {}

extern "C" {
    // Defined in ref/salsa20_ref.c
    static crypto_stream_salsa20_ref_implementation: CryptoStreamSalsa20Implementation;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

// HAVE_AMD64_ASM undefined: ref implementation is selected.
static mut IMPLEMENTATION: *const CryptoStreamSalsa20Implementation =
    core::ptr::addr_of!(crypto_stream_salsa20_ref_implementation);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_keybytes() -> usize {
    CRYPTO_STREAM_SALSA20_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_noncebytes() -> usize {
    CRYPTO_STREAM_SALSA20_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_messagebytes_max() -> usize {
    CRYPTO_STREAM_SALSA20_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    ((*IMPLEMENTATION).stream.unwrap_unchecked())(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    ((*IMPLEMENTATION).stream_xor_ic.unwrap_unchecked())(c, m, mlen, n, ic, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    ((*IMPLEMENTATION).stream_xor_ic.unwrap_unchecked())(c, m, mlen, n, 0u64, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_STREAM_SALSA20_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_stream_salsa20_pick_best_implementation() -> c_int {
    // HAVE_AMD64_ASM undefined and no SIMD HAVE_* macros / runtime dispatch:
    // only the ref implementation is selected.
    IMPLEMENTATION = core::ptr::addr_of!(crypto_stream_salsa20_ref_implementation);
    0 /* LCOV_EXCL_LINE */
}
