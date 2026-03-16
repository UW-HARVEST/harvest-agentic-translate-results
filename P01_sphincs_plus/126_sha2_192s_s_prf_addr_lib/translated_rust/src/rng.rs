use openssl::symm::{Cipher, Crypter, Mode};
use std::sync::Mutex;

pub const RNG_SUCCESS: i32 = 0;
pub const RNG_BAD_MAXLEN: i32 = -1;
pub const RNG_BAD_OUTBUF: i32 = -2;
pub const RNG_BAD_REQ_LEN: i32 = -3;

#[repr(C)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

#[repr(C)]
pub struct Aes256CtrDrbgStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
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
    let count = crypter.update(ctr, buffer).unwrap();
    let _ = crypter.finalize(&mut buffer[count..]);
}

pub fn seedexpander_init(ctx: &mut AesXofStruct, seed: &[u8], diversifier: &[u8], maxlen: u64) -> i32 {
    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }
    ctx.length_remaining = maxlen;
    ctx.key.copy_from_slice(&seed[..32]);
    ctx.ctr[..8].copy_from_slice(&diversifier[..8]);
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

pub fn seedexpander(ctx: &mut AesXofStruct, x: &mut [u8], mut xlen: u64) -> i32 {
    if x.is_empty() {
        return RNG_BAD_OUTBUF;
    }
    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }
    ctx.length_remaining -= xlen;
    let mut offset: usize = 0;

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

pub fn randombytes_init(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    let mut ctx = DRBG_CTX.lock().unwrap();
    ctx.key.fill(0);
    ctx.v.fill(0);
    let mut key = ctx.key;
    let mut v = ctx.v;
    aes256_ctr_drbg_update(Some(&seed_material), &mut key, &mut v);
    ctx.key = key;
    ctx.v = v;
    ctx.reseed_counter = 1;
}

pub fn randombytes(x: &mut [u8], mut xlen: u64) -> i32 {
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    let mut ctx = DRBG_CTX.lock().unwrap();

    while xlen > 0 {
        // increment V
        for j in (0..16).rev() {
            if ctx.v[j] == 0xff {
                ctx.v[j] = 0x00;
            } else {
                ctx.v[j] += 1;
                break;
            }
        }
        aes256_ecb(&ctx.key.clone(), &ctx.v.clone(), &mut block);
        if xlen > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            let xl = xlen as usize;
            x[i..i + xl].copy_from_slice(&block[..xl]);
            xlen = 0;
        }
    }
    let mut key = ctx.key;
    let mut v = ctx.v;
    aes256_ctr_drbg_update(None, &mut key, &mut v);
    ctx.key = key;
    ctx.v = v;
    ctx.reseed_counter += 1;
    RNG_SUCCESS
}

pub fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
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
        aes256_ecb(&key[..], &v[..], &mut temp[16 * i..16 * i + 16]);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}
