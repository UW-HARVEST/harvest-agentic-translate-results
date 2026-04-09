use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;
use crate::params::*;

#[repr(C)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: usize,
    pub length_remaining: usize,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

#[repr(C)]
pub struct Aes256CtrDrbgStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

static mut DRBG_CTX: Aes256CtrDrbgStruct = Aes256CtrDrbgStruct {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
};

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(key.into());
    let mut block = aes::Block::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(&block);
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
    unsafe {
        let k = &*(key as *const [u8; 32]);
        let c = &*(ctr as *const [u8; 16]);
        let b = &mut *(buffer as *mut [u8; 16]);
        aes256_ecb(k, c, b);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut AesXofStruct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: u64,
) -> i32 {
    unsafe {
        if maxlen as u64 >= 0x100000000u64 {
            return RNG_BAD_MAXLEN;
        }

        let c = &mut *ctx;
        c.length_remaining = maxlen as usize;

        std::ptr::copy_nonoverlapping(seed, c.key.as_mut_ptr(), 32);
        std::ptr::copy_nonoverlapping(diversifier, c.ctr.as_mut_ptr(), 8);

        let mut ml = maxlen as u64;
        c.ctr[11] = (ml % 256) as u8;
        ml >>= 8;
        c.ctr[10] = (ml % 256) as u8;
        ml >>= 8;
        c.ctr[9] = (ml % 256) as u8;
        ml >>= 8;
        c.ctr[8] = (ml % 256) as u8;
        std::ptr::write_bytes(c.ctr.as_mut_ptr().add(12), 0, 4);

        c.buffer_pos = 16;
        std::ptr::write_bytes(c.buffer.as_mut_ptr(), 0, 16);

        RNG_SUCCESS
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(ctx: *mut AesXofStruct, x: *mut u8, mut xlen: u64) -> i32 {
    unsafe {
        let c = &mut *ctx;

        if x.is_null() {
            return RNG_BAD_OUTBUF;
        }
        if xlen as usize >= c.length_remaining {
            return RNG_BAD_REQ_LEN;
        }

        c.length_remaining -= xlen as usize;

        let mut offset: usize = 0;
        while xlen > 0 {
            if xlen <= (16 - c.buffer_pos) as u64 {
                std::ptr::copy_nonoverlapping(
                    c.buffer.as_ptr().add(c.buffer_pos),
                    x.add(offset),
                    xlen as usize,
                );
                c.buffer_pos += xlen as usize;
                return RNG_SUCCESS;
            }

            let take = 16 - c.buffer_pos;
            std::ptr::copy_nonoverlapping(
                c.buffer.as_ptr().add(c.buffer_pos),
                x.add(offset),
                take,
            );
            xlen -= take as u64;
            offset += take;

            let mut buf = [0u8; 16];
            aes256_ecb(&c.key, &c.ctr, &mut buf);
            c.buffer = buf;
            c.buffer_pos = 0;

            for i in (12..=15).rev() {
                if c.ctr[i] == 0xff {
                    c.ctr[i] = 0x00;
                } else {
                    c.ctr[i] += 1;
                    break;
                }
            }
        }

        RNG_SUCCESS
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    unsafe {
        let mut temp = [0u8; 48];

        for i in 0..3 {
            // increment V
            for j in (0..=15).rev() {
                if *v.add(j) == 0xff {
                    *v.add(j) = 0x00;
                } else {
                    *v.add(j) += 1;
                    break;
                }
            }

            let mut key_arr = [0u8; 32];
            let mut v_arr = [0u8; 16];
            std::ptr::copy_nonoverlapping(key, key_arr.as_mut_ptr(), 32);
            std::ptr::copy_nonoverlapping(v, v_arr.as_mut_ptr(), 16);
            let mut out = [0u8; 16];
            aes256_ecb(&key_arr, &v_arr, &mut out);
            std::ptr::copy_nonoverlapping(out.as_ptr(), temp.as_mut_ptr().add(16 * i), 16);
        }

        if !provided_data.is_null() {
            for i in 0..48 {
                temp[i] ^= *provided_data.add(i);
            }
        }

        std::ptr::copy_nonoverlapping(temp.as_ptr(), key, 32);
        std::ptr::copy_nonoverlapping(temp.as_ptr().add(32), v, 16);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    unsafe {
        let mut seed_material = [0u8; 48];
        std::ptr::copy_nonoverlapping(entropy_input, seed_material.as_mut_ptr(), 48);

        if !personalization_string.is_null() {
            for i in 0..48 {
                seed_material[i] ^= *personalization_string.add(i);
            }
        }

        std::ptr::write_bytes(std::ptr::addr_of_mut!(DRBG_CTX.key) as *mut u8, 0, 32);
        std::ptr::write_bytes(std::ptr::addr_of_mut!(DRBG_CTX.v) as *mut u8, 0, 16);

        AES256_CTR_DRBG_Update(
            seed_material.as_mut_ptr(),
            std::ptr::addr_of_mut!(DRBG_CTX.key) as *mut u8,
            std::ptr::addr_of_mut!(DRBG_CTX.v) as *mut u8,
        );
        (*std::ptr::addr_of_mut!(DRBG_CTX)).reseed_counter = 1;
    }
}

pub fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    _randombytes(x, xlen)
}

#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub extern "C" fn _randombytes(x: *mut u8, mut xlen: u64) -> i32 {
    unsafe {
        let mut block = [0u8; 16];
        let mut i: usize = 0;

        while xlen > 0 {
            // increment V
            let v_ptr = std::ptr::addr_of_mut!(DRBG_CTX.v) as *mut [u8; 16];
            for j in (0..=15).rev() {
                if (*v_ptr)[j] == 0xff {
                    (*v_ptr)[j] = 0x00;
                } else {
                    (*v_ptr)[j] += 1;
                    break;
                }
            }

            let key_ptr = std::ptr::addr_of!(DRBG_CTX.key) as *const [u8; 32];
            aes256_ecb(&*key_ptr, &*v_ptr, &mut block);

            if xlen > 15 {
                std::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), 16);
                i += 16;
                xlen -= 16;
            } else {
                std::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), xlen as usize);
                xlen = 0;
            }
        }

        AES256_CTR_DRBG_Update(
            std::ptr::null_mut(),
            std::ptr::addr_of_mut!(DRBG_CTX.key) as *mut u8,
            std::ptr::addr_of_mut!(DRBG_CTX.v) as *mut u8,
        );
        (*std::ptr::addr_of_mut!(DRBG_CTX)).reseed_counter += 1;

        RNG_SUCCESS
    }
}
