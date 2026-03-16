use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;
use std::sync::Mutex;

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

#[repr(C)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

pub struct Aes256CtrDrbgStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

static DRBG_CTX: Mutex<Option<Aes256CtrDrbgStruct>> = Mutex::new(None);

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Aes256::new_from_slice(key).unwrap();
    let mut block = aes::Block::default();
    block.copy_from_slice(&ctr[..16]);
    cipher.encrypt_block(&mut block);
    buffer[..16].copy_from_slice(&block);
}

pub fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8], v: &mut [u8]) {
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
        aes256_ecb(key, v, &mut temp[16 * i..]);
    }

    if let Some(data) = provided_data {
        for i in 0..48 {
            temp[i] ^= data[i];
        }
    }

    key[..32].copy_from_slice(&temp[..32]);
    v[..16].copy_from_slice(&temp[32..48]);
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
    let mut offset = 0usize;

    while xlen > 0 {
        let bp = ctx.buffer_pos as usize;
        if xlen <= (16 - bp) as u64 {
            let xl = xlen as usize;
            x[offset..offset + xl].copy_from_slice(&ctx.buffer[bp..bp + xl]);
            ctx.buffer_pos += xlen;
            return RNG_SUCCESS;
        }

        let avail = 16 - bp;
        x[offset..offset + avail].copy_from_slice(&ctx.buffer[bp..16]);
        xlen -= avail as u64;
        offset += avail;

        aes256_ecb(&ctx.key, &ctx.ctr, &mut ctx.buffer);
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

pub fn randombytes_init_internal(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);

    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }

    let mut ctx = Aes256CtrDrbgStruct {
        key: [0u8; 32],
        v: [0u8; 16],
        reseed_counter: 0,
    };

    aes256_ctr_drbg_update(Some(&seed_material), &mut ctx.key, &mut ctx.v);
    ctx.reseed_counter = 1;

    *DRBG_CTX.lock().unwrap() = Some(ctx);
}

pub fn randombytes_internal(x: &mut [u8], mut xlen: u64) -> i32 {
    let mut guard = DRBG_CTX.lock().unwrap();
    let ctx = guard.as_mut().unwrap();

    let mut block = [0u8; 16];
    let mut i = 0usize;

    while xlen > 0 {
        for j in (0..16).rev() {
            if ctx.v[j] == 0xff {
                ctx.v[j] = 0x00;
            } else {
                ctx.v[j] += 1;
                break;
            }
        }
        aes256_ecb(&ctx.key, &ctx.v, &mut block);
        if xlen > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            x[i..i + xlen as usize].copy_from_slice(&block[..xlen as usize]);
            xlen = 0;
        }
    }
    aes256_ctr_drbg_update(None, &mut ctx.key, &mut ctx.v);
    ctx.reseed_counter += 1;

    RNG_SUCCESS
}

pub fn randombytes_urandom(x: &mut [u8], mut xlen: u64) {
    use std::fs::File;
    use std::io::Read;

    let mut f = File::open("/dev/urandom").expect("Failed to open /dev/urandom");
    let mut offset = 0usize;
    while xlen > 0 {
        let to_read = if xlen < 1048576 { xlen as usize } else { 1048576 };
        let n = f.read(&mut x[offset..offset + to_read]).unwrap_or(0);
        if n < 1 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }
        offset += n;
        xlen -= n as u64;
    }
}
