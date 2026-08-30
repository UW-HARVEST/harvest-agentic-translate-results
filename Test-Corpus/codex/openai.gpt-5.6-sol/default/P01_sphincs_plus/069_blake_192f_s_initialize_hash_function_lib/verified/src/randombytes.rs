use aes::Aes256;
use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};
use std::sync::Mutex;

pub const RNG_SUCCESS: i32 = 0;
pub const RNG_BAD_MAXLEN: i32 = -1;
pub const RNG_BAD_OUTBUF: i32 = -2;
pub const RNG_BAD_REQ_LEN: i32 = -3;

#[repr(C)]
pub struct AesXof {
    pub buffer: [u8; 16],
    pub buffer_pos: libc_ulong,
    pub length_remaining: libc_ulong,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

type libc_ulong = std::ffi::c_ulong;

#[repr(C)]
#[derive(Clone)]
pub struct Aes256CtrDrbg {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

impl Default for Aes256CtrDrbg {
    fn default() -> Self {
        Self { key: [0; 32], v: [0; 16], reseed_counter: 0 }
    }
}

#[unsafe(no_mangle)]
pub static mut DRBG_ctx: Aes256CtrDrbg = Aes256CtrDrbg {
    key: [0; 32],
    v: [0; 16],
    reseed_counter: 0,
};

static DRBG_LOCK: Mutex<bool> = Mutex::new(false);

fn increment_be(value: &mut [u8]) {
    for byte in value.iter_mut().rev() {
        if *byte == 0xff {
            *byte = 0;
        } else {
            *byte += 1;
            break;
        }
    }
}

pub fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = GenericArray::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(&block);
}

pub fn aes256_ctr_drbg_update(
    provided_data: Option<&[u8; 48]>,
    key: &mut [u8; 32],
    v: &mut [u8; 16],
) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        increment_be(v);
        aes256_ecb(key, v, (&mut temp[i * 16..(i + 1) * 16]).try_into().unwrap());
    }
    if let Some(data) = provided_data {
        for (dst, src) in temp.iter_mut().zip(data) {
            *dst ^= *src;
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..]);
}

pub fn randombytes_init(entropy_input: &[u8; 48], personalization: Option<&[u8; 48]>) {
    let mut seed_material = *entropy_input;
    if let Some(personalization) = personalization {
        for (seed, personal) in seed_material.iter_mut().zip(personalization) {
            *seed ^= *personal;
        }
    }
    let mut initialized = DRBG_LOCK.lock().unwrap();
    let mut key = [0u8; 32];
    let mut v = [0u8; 16];
    aes256_ctr_drbg_update(Some(&seed_material), &mut key, &mut v);
    unsafe {
        std::ptr::write(
            std::ptr::addr_of_mut!(DRBG_ctx),
            Aes256CtrDrbg { key, v, reseed_counter: 1 },
        );
    }
    *initialized = true;
}

pub fn randombytes(x: &mut [u8], len: usize) {
    let initialized = DRBG_LOCK.lock().unwrap();
    if !*initialized {
        drop(initialized);
        getrandom::fill(&mut x[..len]).expect("failed to read operating-system randomness");
        return;
    }

    let mut state = unsafe { std::ptr::read(std::ptr::addr_of!(DRBG_ctx)) };
    let mut offset = 0;
    while offset < len {
        increment_be(&mut state.v);
        let mut block = [0u8; 16];
        aes256_ecb(&state.key, &state.v, &mut block);
        let take = (len - offset).min(16);
        x[offset..offset + take].copy_from_slice(&block[..take]);
        offset += take;
    }
    let mut key = state.key;
    let mut v = state.v;
    aes256_ctr_drbg_update(None, &mut key, &mut v);
    state.key = key;
    state.v = v;
    state.reseed_counter += 1;
    unsafe {
        std::ptr::write(std::ptr::addr_of_mut!(DRBG_ctx), state);
    }
}

pub fn seedexpander_init(
    ctx: &mut AesXof,
    seed: &[u8; 32],
    diversifier: &[u8; 8],
    mut maxlen: libc_ulong,
) -> i32 {
    if (maxlen as u128) >= 0x1_0000_0000 {
        return RNG_BAD_MAXLEN;
    }
    ctx.length_remaining = maxlen;
    ctx.key.copy_from_slice(seed);
    ctx.ctr[..8].copy_from_slice(diversifier);
    for i in (8..12).rev() {
        ctx.ctr[i] = (maxlen & 0xff) as u8;
        maxlen >>= 8;
    }
    ctx.ctr[12..].fill(0);
    ctx.buffer_pos = 16;
    ctx.buffer.fill(0);
    RNG_SUCCESS
}

pub fn seedexpander(ctx: &mut AesXof, x: &mut [u8], xlen: libc_ulong) -> i32 {
    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }
    ctx.length_remaining -= xlen;
    let mut remaining = xlen as usize;
    let mut offset = 0;
    while remaining > 0 {
        let available = 16 - ctx.buffer_pos as usize;
        if remaining <= available {
            x[offset..offset + remaining].copy_from_slice(
                &ctx.buffer[ctx.buffer_pos as usize..ctx.buffer_pos as usize + remaining],
            );
            ctx.buffer_pos += remaining as libc_ulong;
            return RNG_SUCCESS;
        }
        x[offset..offset + available]
            .copy_from_slice(&ctx.buffer[ctx.buffer_pos as usize..]);
        remaining -= available;
        offset += available;
        aes256_ecb(&ctx.key, &ctx.ctr, &mut ctx.buffer);
        ctx.buffer_pos = 0;
        increment_be(&mut ctx.ctr[12..]);
    }
    RNG_SUCCESS
}
