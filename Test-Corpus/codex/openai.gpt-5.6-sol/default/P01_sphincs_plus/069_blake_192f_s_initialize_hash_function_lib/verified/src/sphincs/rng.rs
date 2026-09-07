use super::aes::encrypt_block;
use std::sync::Mutex;

pub const RNG_SUCCESS: i32 = 0;
pub const RNG_BAD_MAXLEN: i32 = -1;
pub const RNG_BAD_OUTBUF: i32 = -2;
pub const RNG_BAD_REQ_LEN: i32 = -3;

#[repr(C)]
pub struct AesXof {
    pub buffer: [u8; 16],
    pub buffer_pos: usize,
    pub length_remaining: usize,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

#[derive(Clone, Copy)]
struct Drbg {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

static DRBG: Mutex<Drbg> = Mutex::new(Drbg {
    key: [0; 32],
    v: [0; 16],
    reseed_counter: 0,
});

fn increment(v: &mut [u8], first: usize) {
    for i in (first..v.len()).rev() {
        if v[i] == 0xff {
            v[i] = 0;
        } else {
            v[i] += 1;
            break;
        }
    }
}

fn drbg_update(provided: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    for chunk in temp.chunks_exact_mut(16) {
        increment(v, 0);
        chunk.copy_from_slice(&encrypt_block(key, v));
    }
    if let Some(data) = provided {
        for i in 0..48 {
            temp[i] ^= data[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..]);
}

pub fn init(entropy: &[u8; 48], personalization: Option<&[u8; 48]>) {
    let mut material = *entropy;
    if let Some(p) = personalization {
        for i in 0..48 {
            material[i] ^= p[i];
        }
    }
    let mut ctx = DRBG.lock().unwrap();
    ctx.key.fill(0);
    ctx.v.fill(0);
    let mut key = ctx.key;
    let mut v = ctx.v;
    drbg_update(Some(&material), &mut key, &mut v);
    ctx.key = key;
    ctx.v = v;
    ctx.reseed_counter = 1;
}

pub fn fill(out: &mut [u8]) {
    let mut ctx = DRBG.lock().unwrap();
    let mut offset = 0;
    while offset < out.len() {
        increment(&mut ctx.v, 0);
        let block = encrypt_block(&ctx.key, &ctx.v);
        let take = (out.len() - offset).min(16);
        out[offset..offset + take].copy_from_slice(&block[..take]);
        offset += take;
    }
    let mut key = ctx.key;
    let mut v = ctx.v;
    drbg_update(None, &mut key, &mut v);
    ctx.key = key;
    ctx.v = v;
    ctx.reseed_counter += 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
    let key = unsafe { &*(key as *const [u8; 32]) };
    let ctr = unsafe { &*(ctr as *const [u8; 16]) };
    unsafe { std::ptr::copy_nonoverlapping(encrypt_block(key, ctr).as_ptr(), buffer, 16) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    let key = unsafe { &mut *(key as *mut [u8; 32]) };
    let v = unsafe { &mut *(v as *mut [u8; 16]) };
    let provided = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { &*(provided_data as *const [u8; 48]) })
    };
    drbg_update(provided, key, v);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(entropy_input: *mut u8, personalization: *mut u8) {
    let entropy = unsafe { &*(entropy_input as *const [u8; 48]) };
    let p = if personalization.is_null() {
        None
    } else {
        Some(unsafe { &*(personalization as *const [u8; 48]) })
    };
    init(entropy, p);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let out = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    fill(out);
    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut AesXof,
    seed: *mut u8,
    diversifier: *mut u8,
    mut maxlen: usize,
) -> i32 {
    if (maxlen as u128) >= 0x1_0000_0000 {
        return RNG_BAD_MAXLEN;
    }
    let ctx = unsafe { &mut *ctx };
    ctx.length_remaining = maxlen;
    unsafe { std::ptr::copy_nonoverlapping(seed, ctx.key.as_mut_ptr(), 32) };
    unsafe { std::ptr::copy_nonoverlapping(diversifier, ctx.ctr.as_mut_ptr(), 8) };
    for i in (8..12).rev() {
        ctx.ctr[i] = (maxlen & 0xff) as u8;
        maxlen >>= 8;
    }
    ctx.ctr[12..].fill(0);
    ctx.buffer_pos = 16;
    ctx.buffer.fill(0);
    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(ctx: *mut AesXof, x: *mut u8, xlen: usize) -> i32 {
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    let ctx = unsafe { &mut *ctx };
    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }
    ctx.length_remaining -= xlen;
    let out = unsafe { std::slice::from_raw_parts_mut(x, xlen) };
    let mut offset = 0;
    while offset < out.len() {
        if ctx.buffer_pos == 16 {
            ctx.buffer = encrypt_block(&ctx.key, &ctx.ctr);
            ctx.buffer_pos = 0;
            increment(&mut ctx.ctr, 12);
        }
        let take = (16 - ctx.buffer_pos).min(out.len() - offset);
        out[offset..offset + take]
            .copy_from_slice(&ctx.buffer[ctx.buffer_pos..ctx.buffer_pos + take]);
        ctx.buffer_pos += take;
        offset += take;
    }
    RNG_SUCCESS
}
