//! Translation of `app/src/rng.c` + `app/include/rng.h`.
//!
//! This is the NIST KAT AES-256-CTR-DRBG used by the *deterministic*
//! `sphincs_core_det` library variant.  The original C implementation obtains a
//! single-block AES-256 ECB encryption from OpenSSL's `EVP_aes_256_ecb`; here we
//! use the pure-Rust `aes` crate instead.  The observable byte stream is
//! identical: `EVP_EncryptUpdate` over exactly one 16-byte block with no padding
//! emitted is just the raw ECB block encryption.
//!
//! Original header:
//! ```text
//!   rng.c
//!   Created by Bassham, Lawrence E (Fed) on 8/29/17.
//!   Copyright (c) 2017 Bassham, Lawrence E (Fed). All rights reserved.
//! ```

use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
use aes::Aes256;

// #define RNG_SUCCESS      0
// #define RNG_BAD_MAXLEN  -1
// #define RNG_BAD_OUTBUF  -2
// #define RNG_BAD_REQ_LEN -3
pub const RNG_SUCCESS: i32 = 0;
pub const RNG_BAD_MAXLEN: i32 = -1;
pub const RNG_BAD_OUTBUF: i32 = -2;
pub const RNG_BAD_REQ_LEN: i32 = -3;

/// `AES_XOF_struct` from `rng.h`.
///
/// `unsigned long` is 64 bits on the LP64 targets the reference implementation
/// is built for, hence `u64` for `buffer_pos` / `length_remaining`.
#[repr(C)]
#[derive(Clone)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

impl AesXofStruct {
    pub fn new() -> Self {
        AesXofStruct {
            buffer: [0u8; 16],
            buffer_pos: 0,
            length_remaining: 0,
            key: [0u8; 32],
            ctr: [0u8; 16],
        }
    }
}

impl Default for AesXofStruct {
    fn default() -> Self {
        Self::new()
    }
}

/// `AES256_CTR_DRBG_struct` from `rng.h`.
#[repr(C)]
#[derive(Clone)]
pub struct Aes256CtrDrbgStruct {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: i32,
}

impl Aes256CtrDrbgStruct {
    pub fn new() -> Self {
        Aes256CtrDrbgStruct {
            Key: [0u8; 32],
            V: [0u8; 16],
            reseed_counter: 0,
        }
    }
}

impl Default for Aes256CtrDrbgStruct {
    fn default() -> Self {
        Self::new()
    }
}

/// `AES256_CTR_DRBG_struct DRBG_ctx;` -- a zero-initialised file-scope global in
/// the C translation unit.  It is *not* `static` in `rng.c`, so the C
/// `libsphincs_core_det.so` exports it as a writable data symbol named
/// `DRBG_ctx`; we export the very same storage under that exact name so an
/// external consumer can inspect / seed the DRBG state just like with the C
/// library.
#[unsafe(no_mangle)]
pub static mut DRBG_ctx: Aes256CtrDrbgStruct = Aes256CtrDrbgStruct {
    Key: [0u8; 32],
    V: [0u8; 16],
    reseed_counter: 0,
};

/// Borrow the module-global DRBG state, exactly as the C code touches
/// `DRBG_ctx` directly.  Single-threaded use only, matching the C original.
#[inline]
fn drbg_ctx() -> &'static mut Aes256CtrDrbgStruct {
    unsafe { &mut *core::ptr::addr_of_mut!(DRBG_ctx) }
}

// Use whatever AES implementation you have. (The C reference uses OpenSSL.)
//    key    - 256-bit AES key
//    ctr    - a 128-bit plaintext value
//    buffer - a 128-bit ciphertext value
pub fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut blk = *GenericArray::from_slice(ctr);
    cipher.encrypt_block(&mut blk);
    buffer.copy_from_slice(blk.as_slice());
}

/// `seedexpander_init()`
///
/// * `ctx`         - stores the current state of an instance of the seed expander
/// * `seed`        - a 32 byte random value
/// * `diversifier` - an 8 byte diversifier
/// * `maxlen`      - maximum number of bytes (less than 2**32) generated under
///   this seed and diversifier
pub fn seedexpander_init(
    ctx: &mut AesXofStruct,
    seed: &[u8],
    diversifier: &[u8],
    maxlen: u64,
) -> i32 {
    if maxlen >= 0x1_0000_0000u64 {
        return RNG_BAD_MAXLEN;
    }

    ctx.length_remaining = maxlen;

    ctx.key.copy_from_slice(&seed[..32]);

    ctx.ctr[..8].copy_from_slice(&diversifier[..8]);
    let mut maxlen = maxlen;
    ctx.ctr[11] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[10] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[9] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[8] = (maxlen % 256) as u8;
    for b in ctx.ctr[12..16].iter_mut() {
        *b = 0x00;
    }

    ctx.buffer_pos = 16;
    ctx.buffer = [0x00u8; 16];

    RNG_SUCCESS
}

/// `seedexpander()`
///
/// * `ctx` - stores the current state of an instance of the seed expander
/// * `x`   - returns the XOF data (`xlen` in C is `x.len()` here)
///
/// The C `x == NULL` -> `RNG_BAD_OUTBUF` check cannot trigger through this safe
/// entry point; it is preserved in the `extern "C"` wrapper below.
///
/// `ctx.buffer_pos` is caller-controlled state in a caller-allocated struct, so
/// it is NOT necessarily `<= 16`.  The C does `memcpy(x + offset,
/// ctx->buffer + ctx->buffer_pos, n)` with plain pointer arithmetic off the
/// first field of the struct, which for `buffer_pos > 16` happily reads the
/// following struct fields and still returns `RNG_SUCCESS`.  The copies below
/// therefore go through a raw pointer to the whole `AesXofStruct` rather than a
/// bounds-checked index into `ctx.buffer`, so the Rust returns the same bytes
/// and the same code instead of panicking (which, with `panic = "abort"`, would
/// kill the process).
pub fn seedexpander(ctx: &mut AesXofStruct, x: &mut [u8]) -> i32 {
    let mut xlen: u64 = x.len() as u64;

    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }

    ctx.length_remaining -= xlen;

    // `(unsigned char *)&ctx->buffer[0]`, i.e. the base of the struct.
    let ctx_base: *const u8 = ctx as *const AesXofStruct as *const u8;
    let x_base: *mut u8 = x.as_mut_ptr();

    let mut offset: u64 = 0;
    while xlen > 0 {
        // 16 - ctx->buffer_pos  (unsigned long arithmetic, wraps if pos > 16)
        let avail: u64 = 16u64.wrapping_sub(ctx.buffer_pos);

        if xlen <= avail {
            // buffer has what we need
            unsafe {
                core::ptr::copy_nonoverlapping(
                    ctx_base.add(ctx.buffer_pos as usize),
                    x_base.add(offset as usize),
                    xlen as usize,
                );
            }
            ctx.buffer_pos = ctx.buffer_pos.wrapping_add(xlen);

            return RNG_SUCCESS;
        }

        // take what's in the buffer
        unsafe {
            core::ptr::copy_nonoverlapping(
                ctx_base.add(ctx.buffer_pos as usize),
                x_base.add(offset as usize),
                avail as usize,
            );
        }
        xlen -= avail;
        offset += avail;

        let key = ctx.key;
        let ctr = ctx.ctr;
        aes256_ecb(&key, &ctr, &mut ctx.buffer);
        ctx.buffer_pos = 0;

        // increment the counter
        let mut i: i32 = 15;
        while i >= 12 {
            if ctx.ctr[i as usize] == 0xff {
                ctx.ctr[i as usize] = 0x00;
            } else {
                ctx.ctr[i as usize] = ctx.ctr[i as usize].wrapping_add(1);
                break;
            }
            i -= 1;
        }
    }

    RNG_SUCCESS
}

/// `randombytes_init(unsigned char *entropy_input, unsigned char *personalization_string)`
///
/// Note the NIST API variant used here takes no `security_strength` argument.
pub fn randombytes_init(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];

    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }

    let ctx = drbg_ctx();
    ctx.Key = [0x00u8; 32];
    ctx.V = [0x00u8; 16];
    AES256_CTR_DRBG_Update(Some(&seed_material), &mut ctx.Key, &mut ctx.V);
    ctx.reseed_counter = 1;
}

/// `randombytes(unsigned char *x, unsigned long long xlen)`
pub fn randombytes(x: &mut [u8]) -> i32 {
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    let mut xlen: u64 = x.len() as u64;

    let ctx = drbg_ctx();

    while xlen > 0 {
        // increment V
        let mut j: i32 = 15;
        while j >= 0 {
            if ctx.V[j as usize] == 0xff {
                ctx.V[j as usize] = 0x00;
            } else {
                ctx.V[j as usize] = ctx.V[j as usize].wrapping_add(1);
                break;
            }
            j -= 1;
        }

        let key = ctx.Key;
        let v = ctx.V;
        aes256_ecb(&key, &v, &mut block);

        if xlen > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            let n = xlen as usize;
            x[i..i + n].copy_from_slice(&block[..n]);
            xlen = 0;
        }
    }

    AES256_CTR_DRBG_Update(None, &mut ctx.Key, &mut ctx.V);
    ctx.reseed_counter = ctx.reseed_counter.wrapping_add(1);

    RNG_SUCCESS
}

/// `AES256_CTR_DRBG_Update(unsigned char *provided_data, unsigned char *Key, unsigned char *V)`
pub fn AES256_CTR_DRBG_Update(provided_data: Option<&[u8]>, key: &mut [u8], v: &mut [u8]) {
    let mut temp = [0u8; 48];

    for i in 0..3usize {
        // increment V
        let mut j: i32 = 15;
        while j >= 0 {
            if v[j as usize] == 0xff {
                v[j as usize] = 0x00;
            } else {
                v[j as usize] = v[j as usize].wrapping_add(1);
                break;
            }
            j -= 1;
        }

        let mut k = [0u8; 32];
        k.copy_from_slice(&key[..32]);
        let mut c = [0u8; 16];
        c.copy_from_slice(&v[..16]);
        let mut out = [0u8; 16];
        aes256_ecb(&k, &c, &mut out);
        temp[16 * i..16 * i + 16].copy_from_slice(&out);
    }

    if provided_data.is_some() {
        let pd = provided_data.unwrap();
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }

    key[..32].copy_from_slice(&temp[..32]);
    v[..16].copy_from_slice(&temp[32..48]);
}

// ---------------------------------------------------------------------------
// C ABI exports.  `rng.h` declares these with plain (non-namespaced) names --
// there are no `SPX_NAMESPACE` macros in that header -- so the exported symbols
// are the bare C identifiers.  They live in a nested module purely so the Rust
// item names can match the C names exactly without colliding with the safe
// wrappers above.
// ---------------------------------------------------------------------------
pub mod ffi {
    use super::{AesXofStruct, RNG_BAD_OUTBUF};
    use core::ffi::c_int;

    /// C ABI: `void AES256_ECB(unsigned char *key, unsigned char *ctr, unsigned char *buffer)`
    ///
    /// `rng.c` declares this at file scope without `static`, so it is a public
    /// symbol of the C `libsphincs_core_det.so` even though `rng.h` does not
    /// declare it.  key = 256-bit AES key, ctr = 128-bit plaintext block,
    /// buffer = 128-bit ciphertext block.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
        let mut k = [0u8; 32];
        k.copy_from_slice(core::slice::from_raw_parts(key, 32));
        let mut c = [0u8; 16];
        c.copy_from_slice(core::slice::from_raw_parts(ctr, 16));
        let mut out = [0u8; 16];
        super::aes256_ecb(&k, &c, &mut out);
        core::slice::from_raw_parts_mut(buffer, 16).copy_from_slice(&out);
    }

    /// C ABI: `void AES256_CTR_DRBG_Update(unsigned char *provided_data, unsigned char *Key, unsigned char *V)`
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
        provided_data: *mut u8,
        Key: *mut u8,
        V: *mut u8,
    ) {
        let key = core::slice::from_raw_parts_mut(Key, 32);
        let v = core::slice::from_raw_parts_mut(V, 16);
        if provided_data.is_null() {
            super::AES256_CTR_DRBG_Update(None, key, v);
        } else {
            // `provided_data` never aliases Key/V in any caller, mirroring C;
            // copy anyway so the borrow checker is satisfied.
            let mut pd_copy = [0u8; 48];
            pd_copy.copy_from_slice(core::slice::from_raw_parts(provided_data, 48));
            super::AES256_CTR_DRBG_Update(Some(&pd_copy), key, v);
        }
    }

    /// C ABI: `int seedexpander_init(AES_XOF_struct *ctx, unsigned char *seed, unsigned char *diversifier, unsigned long maxlen)`
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn seedexpander_init(
        ctx: *mut AesXofStruct,
        seed: *mut u8,
        diversifier: *mut u8,
        maxlen: u64,
    ) -> c_int {
        let seed_s = core::slice::from_raw_parts(seed, 32);
        let div_s = core::slice::from_raw_parts(diversifier, 8);
        super::seedexpander_init(&mut *ctx, seed_s, div_s, maxlen) as c_int
    }

    /// C ABI: `int seedexpander(AES_XOF_struct *ctx, unsigned char *x, unsigned long xlen)`
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn seedexpander(
        ctx: *mut AesXofStruct,
        x: *mut u8,
        xlen: u64,
    ) -> c_int {
        if x.is_null() {
            return RNG_BAD_OUTBUF as c_int;
        }
        let xs = core::slice::from_raw_parts_mut(x, xlen as usize);
        super::seedexpander(&mut *ctx, xs) as c_int
    }

    /// C ABI: `void randombytes_init(unsigned char *entropy_input, unsigned char *personalization_string)`
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn randombytes_init(
        entropy_input: *mut u8,
        personalization_string: *mut u8,
    ) {
        let mut ei_copy = [0u8; 48];
        ei_copy.copy_from_slice(core::slice::from_raw_parts(entropy_input, 48));
        if personalization_string.is_null() {
            super::randombytes_init(&ei_copy, None);
        } else {
            let mut ps_copy = [0u8; 48];
            ps_copy.copy_from_slice(core::slice::from_raw_parts(personalization_string, 48));
            super::randombytes_init(&ei_copy, Some(&ps_copy));
        }
    }

    /// C ABI: `int randombytes(unsigned char *x, unsigned long long xlen)`
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: u64) -> c_int {
        let xs = core::slice::from_raw_parts_mut(x, xlen as usize);
        super::randombytes(xs) as c_int
    }
}
