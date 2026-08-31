//! Translation of c_src/libsodium/crypto_hash/sha512/hash_sha512.c

// Local repr(C) copy of the public API struct (rule 4), matching
// include/sodium/crypto_hash_sha512.h. No #pragma pack -> plain repr(C).
#[repr(C)]
struct crypto_hash_sha512_state {
    state: [u64; 8],
    count: [u64; 2],
    buf: [u8; 128],
}

// #define crypto_hash_sha512_BYTES 64U
const crypto_hash_sha512_BYTES: usize = 64;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_bytes() -> usize {
    crypto_hash_sha512_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha512_state>()
}

#[allow(dead_code)]
fn _use(s: &crypto_hash_sha512_state) {
    let _ = (&s.state, &s.count, &s.buf);
}
