// Deterministic AES256-CTR-DRBG, matching the NIST rng.c.
//
// Provides:
//   randombytes_init(entropy_input, personalization_string)
//   randombytes(x, xlen) -> int
//
// Uses pure-Rust AES via the `aes` crate (no OpenSSL).

use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

const RNG_SUCCESS: i32 = 0;

#[repr(C)]
pub struct Aes256CtrDrbgStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

static DRBG_INITIALIZED: AtomicBool = AtomicBool::new(false);

fn drbg_lock() -> std::sync::MutexGuard<'static, Aes256CtrDrbgStruct> {
    use std::sync::OnceLock;
    static DRBG: OnceLock<Mutex<Aes256CtrDrbgStruct>> = OnceLock::new();
    DRBG.get_or_init(|| {
        Mutex::new(Aes256CtrDrbgStruct {
            key: [0u8; 32],
            v: [0u8; 16],
            reseed_counter: 0,
        })
    })
    .lock()
    .unwrap()
}

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(key.into());
    let mut block: aes::cipher::generic_array::GenericArray<u8, _> = (*ctr).into();
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(&block);
}

fn drbg_update(provided_data: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
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
        let mut block = [0u8; 16];
        aes256_ecb(key, v, &mut block);
        temp[16 * i..16 * (i + 1)].copy_from_slice(&block);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key.copy_from_slice(&temp[0..32]);
    v.copy_from_slice(&temp[32..48]);
}

pub fn randombytes_init_rs(entropy_input: &[u8; 48], personalization_string: Option<&[u8; 48]>) {
    let mut seed_material = *entropy_input;
    if let Some(p) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= p[i];
        }
    }
    let mut g = drbg_lock();
    g.key = [0u8; 32];
    g.v = [0u8; 16];
    let mut k = g.key;
    let mut v = g.v;
    drbg_update(Some(&seed_material), &mut k, &mut v);
    g.key = k;
    g.v = v;
    g.reseed_counter = 1;
    DRBG_INITIALIZED.store(true, Ordering::SeqCst);
}

pub fn randombytes_rs(out: &mut [u8]) -> i32 {
    let mut g = drbg_lock();
    let mut xlen = out.len();
    let mut i: usize = 0;
    let mut block = [0u8; 16];
    let key = g.key;
    while xlen > 0 {
        // increment V
        for j in (0..16).rev() {
            if g.v[j] == 0xff {
                g.v[j] = 0x00;
            } else {
                g.v[j] += 1;
                break;
            }
        }
        aes256_ecb(&key, &g.v, &mut block);
        if xlen > 15 {
            out[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            out[i..i + xlen].copy_from_slice(&block[..xlen]);
            xlen = 0;
        }
    }
    let mut k = g.key;
    let mut v = g.v;
    drbg_update(None, &mut k, &mut v);
    g.key = k;
    g.v = v;
    g.reseed_counter += 1;
    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let entropy = unsafe { &*(entropy_input as *const [u8; 48]) };
    if personalization_string.is_null() {
        randombytes_init_rs(entropy, None);
    } else {
        let p = unsafe { &*(personalization_string as *const [u8; 48]) };
        randombytes_init_rs(entropy, Some(p));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let slice = unsafe { core::slice::from_raw_parts_mut(x, xlen as usize) };
    randombytes_rs(slice)
}

// AES_XOF_struct definitions and seedexpander functions from rng.h
#[repr(C)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut AesXofStruct,
    seed: *mut u8,
    diversifier: *mut u8,
    mut maxlen: u64,
) -> i32 {
    if maxlen >= 0x100000000u64 {
        return -1;
    }
    let ctx = unsafe { &mut *ctx };
    ctx.length_remaining = maxlen;

    let seed_slice = unsafe { core::slice::from_raw_parts(seed, 32) };
    ctx.key.copy_from_slice(seed_slice);

    let div_slice = unsafe { core::slice::from_raw_parts(diversifier, 8) };
    ctx.ctr[..8].copy_from_slice(div_slice);

    ctx.ctr[11] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[10] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[9] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[8] = (maxlen % 256) as u8;
    for i in 12..16 {
        ctx.ctr[i] = 0;
    }
    ctx.buffer_pos = 16;
    ctx.buffer = [0u8; 16];

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut AesXofStruct,
    x: *mut u8,
    mut xlen: u64,
) -> i32 {
    if x.is_null() {
        return -2;
    }
    let ctx = unsafe { &mut *ctx };
    if xlen >= ctx.length_remaining {
        return -3;
    }
    ctx.length_remaining -= xlen;

    let mut offset: u64 = 0;
    while xlen > 0 {
        if xlen <= (16 - ctx.buffer_pos) {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
                    x.add(offset as usize),
                    xlen as usize,
                );
            }
            ctx.buffer_pos += xlen;
            return 0;
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
                x.add(offset as usize),
                (16 - ctx.buffer_pos) as usize,
            );
        }
        xlen -= 16 - ctx.buffer_pos;
        offset += 16 - ctx.buffer_pos;

        let mut buf = [0u8; 16];
        aes256_ecb(&ctx.key, &ctx.ctr, &mut buf);
        ctx.buffer = buf;
        ctx.buffer_pos = 0;

        for j in (12..16).rev() {
            if ctx.ctr[j] == 0xff {
                ctx.ctr[j] = 0x00;
            } else {
                ctx.ctr[j] += 1;
                break;
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    let key_arr = unsafe { &mut *(key as *mut [u8; 32]) };
    let v_arr = unsafe { &mut *(v as *mut [u8; 16]) };
    if provided_data.is_null() {
        drbg_update(None, key_arr, v_arr);
    } else {
        let pd = unsafe { &*(provided_data as *const [u8; 48]) };
        drbg_update(Some(pd), key_arr, v_arr);
    }
}
