use openssl::symm::{Cipher, Crypter, Mode};
use std::sync::Mutex;

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

struct AesXofStruct {
    buffer: [u8; 16],
    buffer_pos: usize,
    length_remaining: u64,
    key: [u8; 32],
    ctr: [u8; 16],
}

struct Aes256CtrDrbgStruct {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

static DRBG_CTX: Mutex<Aes256CtrDrbgStruct> = Mutex::new(Aes256CtrDrbgStruct {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
});

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Cipher::aes_256_ecb();
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, None).unwrap();
    crypter.pad(false);
    let mut out = [0u8; 32]; // ECB needs at least block size
    let count = crypter.update(ctr, &mut out).unwrap();
    buffer[..16].copy_from_slice(&out[..16]);
    let _ = count;
}

fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];

    for i in 0..3 {
        // increment V
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        aes256_ecb(key, v, &mut temp[16 * i..]);
    }

    if let Some(data) = provided_data {
        for i in 0..48 {
            temp[i] ^= data[i];
        }
    }

    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx_ptr: *mut u8,
    seed: *const u8,
    diversifier: *const u8,
    maxlen: u64,
) -> i32 {
    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }

    // We store AesXofStruct at ctx_ptr as raw bytes
    // For simplicity, use a local struct and write back
    let seed_s = unsafe { core::slice::from_raw_parts(seed, 32) };
    let div_s = unsafe { core::slice::from_raw_parts(diversifier, 8) };

    let mut ctx = AesXofStruct {
        buffer: [0u8; 16],
        buffer_pos: 16,
        length_remaining: maxlen,
        key: [0u8; 32],
        ctr: [0u8; 16],
    };

    ctx.key.copy_from_slice(seed_s);
    ctx.ctr[..8].copy_from_slice(div_s);
    let mut ml = maxlen;
    ctx.ctr[11] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[10] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[9] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[8] = (ml % 256) as u8;
    ctx.ctr[12..16].fill(0);

    // Write ctx to ptr (we use a simple serialization)
    unsafe {
        let ctx_bytes = core::slice::from_raw_parts_mut(ctx_ptr, core::mem::size_of::<AesXofStruct>());
        core::ptr::copy_nonoverlapping(
            &ctx as *const AesXofStruct as *const u8,
            ctx_bytes.as_mut_ptr(),
            core::mem::size_of::<AesXofStruct>(),
        );
    }

    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(ctx_ptr: *mut u8, x: *mut u8, mut xlen: u64) -> i32 {
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }

    let ctx: &mut AesXofStruct = unsafe { &mut *(ctx_ptr as *mut AesXofStruct) };

    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }

    ctx.length_remaining -= xlen;

    let x_slice = unsafe { core::slice::from_raw_parts_mut(x, xlen as usize) };
    let mut offset: usize = 0;

    while xlen > 0 {
        let avail = 16 - ctx.buffer_pos;
        if xlen <= avail as u64 {
            x_slice[offset..offset + xlen as usize]
                .copy_from_slice(&ctx.buffer[ctx.buffer_pos..ctx.buffer_pos + xlen as usize]);
            ctx.buffer_pos += xlen as usize;
            return RNG_SUCCESS;
        }

        x_slice[offset..offset + avail].copy_from_slice(&ctx.buffer[ctx.buffer_pos..16]);
        xlen -= avail as u64;
        offset += avail;

        aes256_ecb(&ctx.key, &ctx.ctr, &mut ctx.buffer);
        ctx.buffer_pos = 0;

        // increment counter
        for i in (12..=15).rev() {
            if ctx.ctr[i] == 0xff {
                ctx.ctr[i] = 0x00;
            } else {
                ctx.ctr[i] += 1;
                break;
            }
        }
    }

    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *const u8,
    personalization_string: *const u8,
) {
    let ei = unsafe { core::slice::from_raw_parts(entropy_input, 48) };
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(ei);

    if !personalization_string.is_null() {
        let ps = unsafe { core::slice::from_raw_parts(personalization_string, 48) };
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }

    let mut ctx = DRBG_CTX.lock().unwrap();
    ctx.key = [0u8; 32];
    ctx.v = [0u8; 16];
    let mut key_copy = ctx.key;
    let mut v_copy = ctx.v;
    aes256_ctr_drbg_update(Some(&seed_material), &mut key_copy, &mut v_copy);
    ctx.key = key_copy;
    ctx.v = v_copy;
    ctx.reseed_counter = 1;
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *const u8,
    key: *mut u8,
    v: *mut u8,
) {
    let key_slice: &mut [u8; 32] = unsafe { &mut *(key as *mut [u8; 32]) };
    let v_slice: &mut [u8; 16] = unsafe { &mut *(v as *mut [u8; 16]) };
    let data = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(provided_data, 48) })
    };
    aes256_ctr_drbg_update(data, key_slice, v_slice);
}
