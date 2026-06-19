use aes::Aes256;
use cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};
use std::fs::File;
use std::io::Read;
use std::sync::{Mutex, OnceLock};

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

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

fn drbg_cell() -> &'static Mutex<AES256_CTR_DRBG_struct> {
    static CELL: OnceLock<Mutex<AES256_CTR_DRBG_struct>> = OnceLock::new();
    CELL.get_or_init(|| {
        Mutex::new(AES256_CTR_DRBG_struct {
            Key: [0; 32],
            V: [0; 16],
            reseed_counter: 0,
        })
    })
}

fn det_enabled() -> &'static Mutex<bool> {
    static CELL: OnceLock<Mutex<bool>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(false))
}

fn urandom_file() -> &'static Mutex<Option<File>> {
    static CELL: OnceLock<Mutex<Option<File>>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(None))
}

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = GenericArray::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(&block);
}

fn drbg_update(provided_data: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0;
            } else {
                v[j] = v[j].wrapping_add(1);
                break;
            }
        }
        let mut block = [0u8; 16];
        aes256_ecb(key, v, &mut block);
        temp[i * 16..(i + 1) * 16].copy_from_slice(&block);
    }
    if let Some(data) = provided_data {
        for i in 0..48 {
            temp[i] ^= data[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

fn randombytes_urandom(out: &mut [u8]) {
    let mut guard = urandom_file().lock().unwrap();
    if guard.is_none() {
        *guard = Some(File::open("/dev/urandom").unwrap());
    }
    let file = guard.as_mut().unwrap();
    file.read_exact(out).unwrap();
}

fn randombytes_det(out: &mut [u8]) {
    let mut ctx = drbg_cell().lock().unwrap();
    let mut offset = 0usize;
    while offset < out.len() {
        for j in (0..16).rev() {
            if ctx.V[j] == 0xff {
                ctx.V[j] = 0;
            } else {
                ctx.V[j] = ctx.V[j].wrapping_add(1);
                break;
            }
        }
        let mut block = [0u8; 16];
        aes256_ecb(&ctx.Key, &ctx.V, &mut block);
        let take = (out.len() - offset).min(16);
        out[offset..offset + take].copy_from_slice(&block[..take]);
        offset += take;
    }
    let mut key = ctx.Key;
    let mut v = ctx.V;
    drbg_update(None, &mut key, &mut v);
    ctx.Key = key;
    ctx.V = v;
    ctx.reseed_counter += 1;
}

fn randombytes_impl(out: &mut [u8]) {
    if *det_enabled().lock().unwrap() {
        randombytes_det(out);
    } else {
        randombytes_urandom(out);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    Key: *mut u8,
    V: *mut u8,
) {
    let key = unsafe { &mut *(Key as *mut [u8; 32]) };
    let v = unsafe { &mut *(V as *mut [u8; 16]) };
    let data = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { &*(provided_data as *const [u8; 48]) })
    };
    drbg_update(data, key, v);
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut AES_XOF_struct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: u64,
) -> i32 {
    if maxlen >= 0x1_0000_0000 {
        return RNG_BAD_MAXLEN;
    }
    let ctx = unsafe { &mut *ctx };
    let seed = unsafe { std::slice::from_raw_parts(seed, 32) };
    let diversifier = unsafe { std::slice::from_raw_parts(diversifier, 8) };
    ctx.length_remaining = maxlen;
    ctx.key.copy_from_slice(seed);
    ctx.ctr[..8].copy_from_slice(diversifier);
    let mut n = maxlen as u32;
    ctx.ctr[11] = (n & 0xff) as u8;
    n >>= 8;
    ctx.ctr[10] = (n & 0xff) as u8;
    n >>= 8;
    ctx.ctr[9] = (n & 0xff) as u8;
    n >>= 8;
    ctx.ctr[8] = (n & 0xff) as u8;
    ctx.ctr[12..].fill(0);
    ctx.buffer_pos = 16;
    ctx.buffer.fill(0);
    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(ctx: *mut AES_XOF_struct, x: *mut u8, xlen: u64) -> i32 {
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    let ctx = unsafe { &mut *ctx };
    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }
    ctx.length_remaining -= xlen;
    let out = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    let mut offset = 0usize;
    let mut remaining = xlen as usize;
    while remaining > 0 {
        let buffer_pos = ctx.buffer_pos as usize;
        let available = 16 - buffer_pos;
        if remaining <= available {
            out[offset..offset + remaining].copy_from_slice(&ctx.buffer[buffer_pos..buffer_pos + remaining]);
            ctx.buffer_pos += remaining as u64;
            return RNG_SUCCESS;
        }
        out[offset..offset + available].copy_from_slice(&ctx.buffer[buffer_pos..]);
        remaining -= available;
        offset += available;
        aes256_ecb(&ctx.key, &ctx.ctr, &mut ctx.buffer);
        ctx.buffer_pos = 0;
        for i in (12..16).rev() {
            if ctx.ctr[i] == 0xff {
                ctx.ctr[i] = 0;
            } else {
                ctx.ctr[i] = ctx.ctr[i].wrapping_add(1);
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
    let personalization = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(personalization_string, 48) })
    };
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(entropy);
    if let Some(p) = personalization {
        for i in 0..48 {
            seed_material[i] ^= p[i];
        }
    }
    let mut ctx = drbg_cell().lock().unwrap();
    ctx.Key.fill(0);
    ctx.V.fill(0);
    let mut key = ctx.Key;
    let mut v = ctx.V;
    drbg_update(Some(&seed_material), &mut key, &mut v);
    ctx.Key = key;
    ctx.V = v;
    ctx.reseed_counter = 1;
    *det_enabled().lock().unwrap() = true;
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let out = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    randombytes_impl(out);
    RNG_SUCCESS
}
