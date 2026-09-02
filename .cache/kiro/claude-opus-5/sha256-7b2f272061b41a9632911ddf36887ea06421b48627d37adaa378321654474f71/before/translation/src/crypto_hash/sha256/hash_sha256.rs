//! Translation of c_src/libsodium/crypto_hash/sha256/hash_sha256.c

use core::ffi::c_int;

// Local repr(C) copy of the public API struct (rule 4), matching
// include/sodium/crypto_hash_sha256.h. No #pragma pack -> plain repr(C).
#[repr(C)]
struct crypto_hash_sha256_state {
    state: [u32; 8],
    count: u64,
    buf: [u8; 64],
}

// #define crypto_hash_sha256_BYTES 32U
const crypto_hash_sha256_BYTES: usize = 32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_bytes() -> usize {
    crypto_hash_sha256_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha256_state>()
}

// Silence dead-field lints without changing layout.
#[allow(dead_code)]
fn _use(s: &crypto_hash_sha256_state) -> c_int {
    let _ = (&s.state, &s.count, &s.buf);
    0
}
