// RNG state (global mutable, matching C's global DRBG_ctx)
static mut DRBG_CTX: AES256CtrDrbgStruct = AES256CtrDrbgStruct {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
};

struct AES256CtrDrbgStruct {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

// AES-256-ECB using OpenSSL via FFI
extern "C" {
    fn EVP_CIPHER_CTX_new() -> *mut std::ffi::c_void;
    fn EVP_CIPHER_CTX_free(ctx: *mut std::ffi::c_void);
    fn EVP_aes_256_ecb() -> *const std::ffi::c_void;
    fn EVP_EncryptInit_ex(
        ctx: *mut std::ffi::c_void,
        cipher: *const std::ffi::c_void,
        engine: *const std::ffi::c_void,
        key: *const u8,
        iv: *const u8,
    ) -> i32;
    fn EVP_EncryptUpdate(
        ctx: *mut std::ffi::c_void,
        out: *mut u8,
        outl: *mut i32,
        inp: *const u8,
        inl: i32,
    ) -> i32;
    fn ERR_print_errors_fp(fp: *mut std::ffi::c_void);
}

extern "C" {
    fn fdopen(fd: i32, mode: *const u8) -> *mut std::ffi::c_void;
}

fn handle_errors() {
    unsafe {
        let stderr = fdopen(2, b"w\0".as_ptr());
        ERR_print_errors_fp(stderr);
    }
    std::process::abort();
}

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    unsafe {
        let ctx = EVP_CIPHER_CTX_new();
        if ctx.is_null() { handle_errors(); }
        if EVP_EncryptInit_ex(ctx, EVP_aes_256_ecb(), std::ptr::null(), key.as_ptr(), std::ptr::null()) != 1 {
            handle_errors();
        }
        let mut len: i32 = 0;
        if EVP_EncryptUpdate(ctx, buffer.as_mut_ptr(), &mut len, ctr.as_ptr(), 16) != 1 {
            handle_errors();
        }
        EVP_CIPHER_CTX_free(ctx);
    }
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

pub fn randombytes_init(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    unsafe {
        DRBG_CTX.key = [0u8; 32];
        DRBG_CTX.v = [0u8; 16];
        aes256_ctr_drbg_update(Some(&seed_material), &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter = 1;
    }
}

pub fn randombytes(x: &mut [u8], mut xlen: u64) {
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    let xlen_usize = xlen as usize;

    unsafe {
        while xlen > 0 {
            // increment V
            for j in (0..16).rev() {
                if DRBG_CTX.v[j] == 0xff {
                    DRBG_CTX.v[j] = 0x00;
                } else {
                    DRBG_CTX.v[j] += 1;
                    break;
                }
            }
            aes256_ecb(&DRBG_CTX.key, &DRBG_CTX.v, &mut block);
            if xlen > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16;
                xlen -= 16;
            } else {
                x[i..i + xlen as usize].copy_from_slice(&block[..xlen as usize]);
                xlen = 0;
            }
        }
        aes256_ctr_drbg_update(None, &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter += 1;
    }
}
