use std::sync::Mutex;

// RNG constants
pub const RNG_SUCCESS: i32 = 0;
pub const RNG_BAD_MAXLEN: i32 = -1;
pub const RNG_BAD_OUTBUF: i32 = -2;
pub const RNG_BAD_REQ_LEN: i32 = -3;

#[repr(C)]
pub struct AES_XOF_struct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

#[repr(C)]
pub struct AES256_CTR_DRBG_struct {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: i32,
}

static DRBG_CTX: Mutex<AES256_CTR_DRBG_struct> = Mutex::new(AES256_CTR_DRBG_struct {
    Key: [0u8; 32],
    V: [0u8; 16],
    reseed_counter: 0,
});

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    use openssl::symm::{Cipher, Crypter, Mode};
    let cipher = Cipher::aes_256_ecb();
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, None).unwrap();
    crypter.pad(false);
    let count = crypter.update(ctr, buffer).unwrap();
    let _ = crypter.finalize(&mut buffer[count..]).unwrap();
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut AES_XOF_struct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: u64,
) -> i32 {
    let ctx = unsafe { &mut *ctx };
    let seed = unsafe { std::slice::from_raw_parts(seed, 32) };
    let diversifier = unsafe { std::slice::from_raw_parts(diversifier, 8) };

    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }

    ctx.length_remaining = maxlen;
    ctx.key.copy_from_slice(seed);
    ctx.ctr[..8].copy_from_slice(diversifier);

    let mut ml = maxlen;
    ctx.ctr[11] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[10] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[9] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[8] = (ml % 256) as u8;
    ctx.ctr[12..16].fill(0);

    ctx.buffer_pos = 16;
    ctx.buffer.fill(0);

    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(ctx: *mut AES_XOF_struct, x: *mut u8, xlen: u64) -> i32 {
    let ctx = unsafe { &mut *ctx };
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    let mut xlen = xlen;
    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }

    ctx.length_remaining -= xlen;
    let x = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };

    let mut offset = 0usize;
    while xlen > 0 {
        let bp = ctx.buffer_pos as usize;
        if xlen <= (16 - bp) as u64 {
            let xl = xlen as usize;
            x[offset..offset + xl].copy_from_slice(&ctx.buffer[bp..bp + xl]);
            ctx.buffer_pos += xlen;
            return RNG_SUCCESS;
        }

        let take = 16 - bp;
        x[offset..offset + take].copy_from_slice(&ctx.buffer[bp..16]);
        xlen -= take as u64;
        offset += take;

        aes256_ecb(&ctx.key, &ctx.ctr.clone(), &mut ctx.buffer);
        ctx.buffer_pos = 0;

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
pub extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let entropy = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(entropy);

    if !personalization_string.is_null() {
        let ps = unsafe { std::slice::from_raw_parts(personalization_string, 48) };
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }

    let mut ctx = DRBG_CTX.lock().unwrap();
    ctx.Key.fill(0);
    ctx.V.fill(0);

    let mut key_copy = ctx.Key;
    let mut v_copy = ctx.V;
    aes256_ctr_drbg_update_inner(Some(&seed_material), &mut key_copy, &mut v_copy);
    ctx.Key = key_copy;
    ctx.V = v_copy;
    ctx.reseed_counter = 1;
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let mut ctx = DRBG_CTX.lock().unwrap();
    let x = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    let mut block = [0u8; 16];
    let mut i = 0usize;
    let mut remaining = xlen as usize;

    while remaining > 0 {
        // increment V
        for j in (0..16).rev() {
            if ctx.V[j] == 0xff {
                ctx.V[j] = 0x00;
            } else {
                ctx.V[j] += 1;
                break;
            }
        }
        aes256_ecb(&ctx.Key, &ctx.V.clone(), &mut block);
        if remaining > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            remaining -= 16;
        } else {
            x[i..i + remaining].copy_from_slice(&block[..remaining]);
            remaining = 0;
        }
    }

    let mut key_copy = ctx.Key;
    let mut v_copy = ctx.V;
    aes256_ctr_drbg_update_inner(None, &mut key_copy, &mut v_copy);
    ctx.Key = key_copy;
    ctx.V = v_copy;
    ctx.reseed_counter += 1;

    RNG_SUCCESS
}

fn aes256_ctr_drbg_update_inner(
    provided_data: Option<&[u8]>,
    key: &mut [u8; 32],
    v: &mut [u8; 16],
) {
    let mut temp = [0u8; 48];

    for i in 0..3 {
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        aes256_ecb(key, &v.clone(), &mut temp[16 * i..16 * i + 16]);
    }

    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }

    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    Key: *mut u8,
    V: *mut u8,
) {
    let key = unsafe { &mut *(Key as *mut [u8; 32]) };
    let v = unsafe { &mut *(V as *mut [u8; 16]) };

    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(provided_data, 48) })
    };

    aes256_ctr_drbg_update_inner(pd, key, v);
}
