//! Translation of `app/src/rng.c` / `app/include/rng.h` — the NIST
//! `AES256_CTR_DRBG` used by the KAT generator.
//!
//! The C file obtains its single AES-256-ECB block encryption from OpenSSL's
//! EVP interface (`EVP_aes_256_ecb`).  Since this crate must not depend on any
//! external code, AES-256 block encryption is implemented here in pure safe
//! Rust (FIPS-197).  It is bit-exact with `EVP_aes_256_ecb` for a single
//! 16-byte block without padding, which is all `AES256_ECB()` ever asks for.

// ---------------------------------------------------------------------------
// rng.h
// ---------------------------------------------------------------------------

/// `#define RNG_SUCCESS 0`
pub const RNG_SUCCESS: i32 = 0;
/// `#define RNG_BAD_MAXLEN -1`
pub const RNG_BAD_MAXLEN: i32 = -1;
/// `#define RNG_BAD_OUTBUF -2`
pub const RNG_BAD_OUTBUF: i32 = -2;
/// `#define RNG_BAD_REQ_LEN -3`
pub const RNG_BAD_REQ_LEN: i32 = -3;

/// ```c
/// typedef struct {
///     unsigned char   buffer[16];
///     unsigned long   buffer_pos;
///     unsigned long   length_remaining;
///     unsigned char   key[32];
///     unsigned char   ctr[16];
/// } AES_XOF_struct;
/// ```
#[repr(C)]
pub struct AES_XOF_struct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

/// ```c
/// typedef struct {
///     unsigned char   Key[32];
///     unsigned char   V[16];
///     int             reseed_counter;
/// } AES256_CTR_DRBG_struct;
/// ```
#[repr(C)]
pub struct AES256_CTR_DRBG_struct {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: i32,
}

// ---------------------------------------------------------------------------
// AES-256 (FIPS-197), replacement for the OpenSSL EVP calls
// ---------------------------------------------------------------------------

/// FIPS-197 Figure 7: the AES S-box.
const AES_SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

/// FIPS-197 Section 5.2: `Rcon[i]`, i.e. `x^(i-1)` in GF(2^8) (the leading
/// byte of the round constant word).  Index 0 is never used.
const AES_RCON: [u8; 11] = [
    0x8d, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36,
];

/// AES-256 uses `Nr = 14` rounds, i.e. 15 round keys of 16 bytes each.
const AES256_ROUNDS: usize = 14;
/// `4 * 4 * (Nr + 1)` = 240 round-key bytes.
const AES256_RK_BYTES: usize = 16 * (AES256_ROUNDS + 1);

/// Multiplication by `x` (i.e. `0x02`) in GF(2^8) modulo the AES polynomial.
#[inline]
fn xtime(a: u8) -> u8 {
    (a << 1) ^ (((a >> 7) & 1) * 0x1b)
}

/// FIPS-197 Section 5.2 `KeyExpansion()` for a 32-byte (256-bit) key:
/// `Nk = 8`, `Nr = 14`, producing 60 words / 240 bytes of round keys.
fn aes256_key_expansion(key: &[u8; 32]) -> [u8; AES256_RK_BYTES] {
    let mut rk = [0u8; AES256_RK_BYTES];
    rk[..32].copy_from_slice(key);

    let mut i = 32;
    while i < AES256_RK_BYTES {
        // temp = w[i-1]
        let mut t = [rk[i - 4], rk[i - 3], rk[i - 2], rk[i - 1]];

        if i % 32 == 0 {
            // temp = SubWord(RotWord(temp)) xor Rcon[i/Nk]
            let t0 = t[0];
            t[0] = AES_SBOX[t[1] as usize] ^ AES_RCON[i / 32];
            t[1] = AES_SBOX[t[2] as usize];
            t[2] = AES_SBOX[t[3] as usize];
            t[3] = AES_SBOX[t0 as usize];
        } else if i % 32 == 16 {
            // Nk > 6 and i mod Nk == 4: temp = SubWord(temp)
            t[0] = AES_SBOX[t[0] as usize];
            t[1] = AES_SBOX[t[1] as usize];
            t[2] = AES_SBOX[t[2] as usize];
            t[3] = AES_SBOX[t[3] as usize];
        }

        // w[i] = w[i-Nk] xor temp
        rk[i] = rk[i - 32] ^ t[0];
        rk[i + 1] = rk[i - 31] ^ t[1];
        rk[i + 2] = rk[i - 30] ^ t[2];
        rk[i + 3] = rk[i - 29] ^ t[3];

        i += 4;
    }

    rk
}

/// `AddRoundKey()`: the state is held in column-major order, exactly the byte
/// order of the input block, so the round key bytes line up one-to-one.
#[inline]
fn add_round_key(state: &mut [u8; 16], rk: &[u8; AES256_RK_BYTES], round: usize) {
    let base = 16 * round;
    for j in 0..16 {
        state[j] ^= rk[base + j];
    }
}

/// `SubBytes()`
#[inline]
fn sub_bytes(state: &mut [u8; 16]) {
    for b in state.iter_mut() {
        *b = AES_SBOX[*b as usize];
    }
}

/// `ShiftRows()`: row `r` is rotated left by `r` positions.
#[inline]
fn shift_rows(state: &mut [u8; 16]) {
    let s = *state;
    for c in 0..4 {
        for r in 0..4 {
            state[4 * c + r] = s[4 * ((c + r) % 4) + r];
        }
    }
}

/// `MixColumns()`
#[inline]
fn mix_columns(state: &mut [u8; 16]) {
    for c in 0..4 {
        let a0 = state[4 * c];
        let a1 = state[4 * c + 1];
        let a2 = state[4 * c + 2];
        let a3 = state[4 * c + 3];
        let t = a0 ^ a1 ^ a2 ^ a3;

        state[4 * c] = a0 ^ t ^ xtime(a0 ^ a1);
        state[4 * c + 1] = a1 ^ t ^ xtime(a1 ^ a2);
        state[4 * c + 2] = a2 ^ t ^ xtime(a2 ^ a3);
        state[4 * c + 3] = a3 ^ t ^ xtime(a3 ^ a0);
    }
}

/// FIPS-197 Section 5.1 `Cipher()` for a single 16-byte block, `Nr = 14`.
fn aes256_encrypt_block(rk: &[u8; AES256_RK_BYTES], input: &[u8; 16]) -> [u8; 16] {
    let mut state = *input;

    add_round_key(&mut state, rk, 0);

    for round in 1..AES256_ROUNDS {
        sub_bytes(&mut state);
        shift_rows(&mut state);
        mix_columns(&mut state);
        add_round_key(&mut state, rk, round);
    }

    sub_bytes(&mut state);
    shift_rows(&mut state);
    add_round_key(&mut state, rk, AES256_ROUNDS);

    state
}

// ---------------------------------------------------------------------------
// rng.c
// ---------------------------------------------------------------------------

/// `AES256_CTR_DRBG_struct DRBG_ctx;` — the file-scope DRBG state of `rng.c`
/// (a C global with external linkage, hence zero initialised and exported by
/// `libsphincs_core_det.so` as the data symbol `DRBG_ctx`).
#[unsafe(no_mangle)]
pub static mut DRBG_ctx: AES256_CTR_DRBG_struct = AES256_CTR_DRBG_struct {
    Key: [0u8; 32],
    V: [0u8; 16],
    reseed_counter: 0,
};

/// ```c
/// static void handleErrors(void)
/// {
///     ERR_print_errors_fp(stderr);
///     abort();
/// }
/// ```
///
/// There is no OpenSSL error queue to print here, so only the `abort()`
/// remains.  With the in-crate AES this is unreachable, just as it is in
/// practice in C.
#[allow(dead_code)]
fn handleErrors() -> ! {
    std::process::abort()
}

/// ```c
/// void AES256_ECB(unsigned char *key, unsigned char *ctr, unsigned char *buffer)
/// ```
///
/// * `key`    - 256-bit AES key
/// * `ctr`    - a 128-bit plaintext value
/// * `buffer` - a 128-bit ciphertext value
///
/// The OpenSSL EVP dance (`EVP_CIPHER_CTX_new` / `EVP_EncryptInit_ex` with
/// `EVP_aes_256_ecb` / `EVP_EncryptUpdate` over 16 bytes / `EVP_CIPHER_CTX_free`)
/// is exactly one AES-256 block encryption; none of the steps can fail here,
/// so `handleErrors()` is never reached.
/// (In C this function has external linkage, so it is exported under its plain
/// name — rng.h does not apply the `SPX_NAMESPACE` macro to it.)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
    let mut key_bytes = [0u8; 32];
    core::ptr::copy_nonoverlapping(key, key_bytes.as_mut_ptr(), 32);

    let mut block = [0u8; 16];
    core::ptr::copy_nonoverlapping(ctr, block.as_mut_ptr(), 16);

    let rk = aes256_key_expansion(&key_bytes);
    let out = aes256_encrypt_block(&rk, &block);

    core::ptr::copy_nonoverlapping(out.as_ptr(), buffer, 16);
}

/// ```c
/// int seedexpander_init(AES_XOF_struct *ctx, unsigned char *seed,
///                       unsigned char *diversifier, unsigned long maxlen)
/// ```
///
/// * `ctx`         - stores the current state of an instance of the seed expander
/// * `seed`        - a 32 byte random value
/// * `diversifier` - an 8 byte diversifier
/// * `maxlen`      - maximum number of bytes (less than 2**32) generated under
///   this seed and diversifier
#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut AES_XOF_struct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: u64,
) -> i32 {
    let mut maxlen = maxlen;

    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }

    (*ctx).length_remaining = maxlen;

    core::ptr::copy_nonoverlapping(seed, (*ctx).key.as_mut_ptr(), 32);

    core::ptr::copy_nonoverlapping(diversifier, (*ctx).ctr.as_mut_ptr(), 8);
    (*ctx).ctr[11] = (maxlen % 256) as u8;
    maxlen >>= 8;
    (*ctx).ctr[10] = (maxlen % 256) as u8;
    maxlen >>= 8;
    (*ctx).ctr[9] = (maxlen % 256) as u8;
    maxlen >>= 8;
    (*ctx).ctr[8] = (maxlen % 256) as u8;
    core::ptr::write_bytes((*ctx).ctr.as_mut_ptr().add(12), 0x00, 4);

    (*ctx).buffer_pos = 16;
    core::ptr::write_bytes((*ctx).buffer.as_mut_ptr(), 0x00, 16);

    RNG_SUCCESS
}

/// ```c
/// int seedexpander(AES_XOF_struct *ctx, unsigned char *x, unsigned long xlen)
/// ```
///
/// * `ctx`  - stores the current state of an instance of the seed expander
/// * `x`    - returns the XOF data
/// * `xlen` - number of bytes to return
#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(ctx: *mut AES_XOF_struct, x: *mut u8, xlen: u64) -> i32 {
    let mut xlen = xlen;
    let mut offset: u64;

    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    if xlen >= (*ctx).length_remaining {
        return RNG_BAD_REQ_LEN;
    }

    (*ctx).length_remaining -= xlen;

    offset = 0;
    while xlen > 0 {
        // `16 - ctx->buffer_pos` is unsigned long arithmetic in C.
        let avail = 16u64.wrapping_sub((*ctx).buffer_pos);

        if xlen <= avail {
            // buffer has what we need
            core::ptr::copy_nonoverlapping(
                (*ctx).buffer.as_ptr().add((*ctx).buffer_pos as usize),
                x.add(offset as usize),
                xlen as usize,
            );
            (*ctx).buffer_pos += xlen;

            return RNG_SUCCESS;
        }

        // take what's in the buffer
        core::ptr::copy_nonoverlapping(
            (*ctx).buffer.as_ptr().add((*ctx).buffer_pos as usize),
            x.add(offset as usize),
            avail as usize,
        );
        xlen -= avail;
        offset += avail;

        let key = (*ctx).key.as_mut_ptr();
        let ctr = (*ctx).ctr.as_mut_ptr();
        let buffer = (*ctx).buffer.as_mut_ptr();
        AES256_ECB(key, ctr, buffer);
        (*ctx).buffer_pos = 0;

        //increment the counter
        let mut i: i32 = 15;
        while i >= 12 {
            if (*ctx).ctr[i as usize] == 0xff {
                (*ctx).ctr[i as usize] = 0x00;
            } else {
                (*ctx).ctr[i as usize] += 1;
                break;
            }
            i -= 1;
        }
    }

    RNG_SUCCESS
}

/// ```c
/// void randombytes_init(unsigned char *entropy_input,
///                       unsigned char *personalization_string)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(entropy_input: *mut u8, personalization_string: *mut u8) {
    let mut seed_material = [0u8; 48];

    core::ptr::copy_nonoverlapping(entropy_input, seed_material.as_mut_ptr(), 48);
    if !personalization_string.is_null() {
        for i in 0..48 {
            seed_material[i] ^= *personalization_string.add(i);
        }
    }
    let ctx = &raw mut DRBG_ctx;
    core::ptr::write_bytes((*ctx).Key.as_mut_ptr(), 0x00, 32);
    core::ptr::write_bytes((*ctx).V.as_mut_ptr(), 0x00, 16);
    AES256_CTR_DRBG_Update(
        seed_material.as_mut_ptr(),
        (*ctx).Key.as_mut_ptr(),
        (*ctx).V.as_mut_ptr(),
    );
    (*ctx).reseed_counter = 1;
}

/// ```c
/// int randombytes(unsigned char *x, unsigned long long xlen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let mut xlen = xlen;
    let mut block = [0u8; 16];
    let mut i: i32 = 0;

    let ctx = &raw mut DRBG_ctx;

    while xlen > 0 {
        //increment V
        let mut j: i32 = 15;
        while j >= 0 {
            if (*ctx).V[j as usize] == 0xff {
                (*ctx).V[j as usize] = 0x00;
            } else {
                (*ctx).V[j as usize] += 1;
                break;
            }
            j -= 1;
        }
        AES256_ECB(
            (*ctx).Key.as_mut_ptr(),
            (*ctx).V.as_mut_ptr(),
            block.as_mut_ptr(),
        );
        if xlen > 15 {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i as usize), 16);
            i += 16;
            xlen -= 16;
        } else {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i as usize), xlen as usize);
            xlen = 0;
        }
    }
    AES256_CTR_DRBG_Update(
        core::ptr::null_mut(),
        (*ctx).Key.as_mut_ptr(),
        (*ctx).V.as_mut_ptr(),
    );
    (*ctx).reseed_counter += 1;

    RNG_SUCCESS
}

/// ```c
/// void AES256_CTR_DRBG_Update(unsigned char *provided_data,
///                             unsigned char *Key, unsigned char *V)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(provided_data: *mut u8, Key: *mut u8, V: *mut u8) {
    let mut temp = [0u8; 48];

    for i in 0..3 {
        //increment V
        let mut j: i32 = 15;
        while j >= 0 {
            if *V.add(j as usize) == 0xff {
                *V.add(j as usize) = 0x00;
            } else {
                *V.add(j as usize) += 1;
                break;
            }
            j -= 1;
        }

        AES256_ECB(Key, V, temp.as_mut_ptr().add(16 * i));
    }
    if !provided_data.is_null() {
        for i in 0..48 {
            temp[i] ^= *provided_data.add(i);
        }
    }
    core::ptr::copy_nonoverlapping(temp.as_ptr(), Key, 32);
    core::ptr::copy_nonoverlapping(temp.as_ptr().add(32), V, 16);
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// FIPS-197, Appendix C.3 — AES-256 (Nk = 8, Nr = 14).
    #[test]
    fn aes256_fips197_appendix_c3() {
        let mut key = [0u8; 32];
        for (i, b) in key.iter_mut().enumerate() {
            *b = i as u8; // 000102...1f
        }
        let plaintext: [u8; 16] = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let expected: [u8; 16] = [
            0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b, 0x49,
            0x60, 0x89,
        ];

        let rk = aes256_key_expansion(&key);
        assert_eq!(aes256_encrypt_block(&rk, &plaintext), expected);

        // Same vector through the `AES256_ECB()` wrapper used by the DRBG.
        let mut ctr = plaintext;
        let mut buffer = [0u8; 16];
        unsafe {
            AES256_ECB(key.as_mut_ptr(), ctr.as_mut_ptr(), buffer.as_mut_ptr());
        }
        assert_eq!(buffer, expected);
    }
}
