extern "C" {
    pub type evp_cipher_st;
    pub type evp_cipher_ctx_st;
    pub type engine_st;
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    static mut stderr: *mut _IO_FILE;
    fn abort() -> !;
    fn EVP_EncryptInit_ex(
        ctx: *mut EVP_CIPHER_CTX,
        cipher: *const EVP_CIPHER,
        impl_0: *mut ENGINE,
        key: *const libc::c_uchar,
        iv: *const libc::c_uchar,
    ) -> libc::c_int;
    fn EVP_EncryptUpdate(
        ctx: *mut EVP_CIPHER_CTX,
        out: *mut libc::c_uchar,
        outl: *mut libc::c_int,
        in_0: *const libc::c_uchar,
        inl: libc::c_int,
    ) -> libc::c_int;
    fn EVP_CIPHER_CTX_new() -> *mut EVP_CIPHER_CTX;
    fn EVP_CIPHER_CTX_free(c: *mut EVP_CIPHER_CTX);
    fn EVP_aes_256_ecb() -> *const EVP_CIPHER;
    fn ERR_print_errors_fp(fp: *mut FILE);
}
pub type size_t = usize;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
pub type EVP_CIPHER = evp_cipher_st;
pub type EVP_CIPHER_CTX = evp_cipher_ctx_st;
pub type ENGINE = engine_st;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct AES_XOF_struct {
    pub buffer: [libc::c_uchar; 16],
    pub buffer_pos: libc::c_ulong,
    pub length_remaining: libc::c_ulong,
    pub key: [libc::c_uchar; 32],
    pub ctr: [libc::c_uchar; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct AES256_CTR_DRBG_struct {
    pub Key: [libc::c_uchar; 32],
    pub V: [libc::c_uchar; 16],
    pub reseed_counter: libc::c_int,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const RNG_SUCCESS: libc::c_int = 0 as libc::c_int;
pub const RNG_BAD_MAXLEN: libc::c_int = -(1 as libc::c_int);
pub const RNG_BAD_OUTBUF: libc::c_int = -(2 as libc::c_int);
pub const RNG_BAD_REQ_LEN: libc::c_int = -(3 as libc::c_int);
#[no_mangle]
pub static mut DRBG_ctx: AES256_CTR_DRBG_struct = AES256_CTR_DRBG_struct {
    Key: [0; 32],
    V: [0; 16],
    reseed_counter: 0,
};
#[no_mangle]
pub unsafe extern "C" fn seedexpander_init(
    mut ctx: *mut AES_XOF_struct,
    mut seed: *mut libc::c_uchar,
    mut diversifier: *mut libc::c_uchar,
    mut maxlen: libc::c_ulong,
) -> libc::c_int {
    if maxlen >= 0x100000000 as libc::c_long as libc::c_ulong {
        return RNG_BAD_MAXLEN;
    }
    (*ctx).length_remaining = maxlen;
    memcpy(
        &raw mut (*ctx).key as *mut libc::c_uchar as *mut libc::c_void,
        seed as *const libc::c_void,
        32 as size_t,
    );
    memcpy(
        &raw mut (*ctx).ctr as *mut libc::c_uchar as *mut libc::c_void,
        diversifier as *const libc::c_void,
        8 as size_t,
    );
    (*ctx).ctr[11 as libc::c_int as usize] =
        maxlen.wrapping_rem(256 as libc::c_ulong) as libc::c_uchar;
    maxlen >>= 8 as libc::c_int;
    (*ctx).ctr[10 as libc::c_int as usize] =
        maxlen.wrapping_rem(256 as libc::c_ulong) as libc::c_uchar;
    maxlen >>= 8 as libc::c_int;
    (*ctx).ctr[9 as libc::c_int as usize] =
        maxlen.wrapping_rem(256 as libc::c_ulong) as libc::c_uchar;
    maxlen >>= 8 as libc::c_int;
    (*ctx).ctr[8 as libc::c_int as usize] =
        maxlen.wrapping_rem(256 as libc::c_ulong) as libc::c_uchar;
    memset(
        (&raw mut (*ctx).ctr as *mut libc::c_uchar).offset(12 as libc::c_int as isize)
            as *mut libc::c_void,
        0 as libc::c_int,
        4 as size_t,
    );
    (*ctx).buffer_pos = 16 as libc::c_ulong;
    memset(
        &raw mut (*ctx).buffer as *mut libc::c_uchar as *mut libc::c_void,
        0 as libc::c_int,
        16 as size_t,
    );
    return RNG_SUCCESS;
}
#[no_mangle]
pub unsafe extern "C" fn seedexpander(
    mut ctx: *mut AES_XOF_struct,
    mut x: *mut libc::c_uchar,
    mut xlen: libc::c_ulong,
) -> libc::c_int {
    let mut offset: libc::c_ulong = 0;
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    if xlen >= (*ctx).length_remaining {
        return RNG_BAD_REQ_LEN;
    }
    (*ctx).length_remaining = (*ctx).length_remaining.wrapping_sub(xlen);
    offset = 0 as libc::c_ulong;
    while xlen > 0 as libc::c_ulong {
        if xlen <= (16 as libc::c_ulong).wrapping_sub((*ctx).buffer_pos) {
            memcpy(
                x.offset(offset as isize) as *mut libc::c_void,
                (&raw mut (*ctx).buffer as *mut libc::c_uchar)
                    .offset((*ctx).buffer_pos as isize)
                    as *const libc::c_void,
                xlen as size_t,
            );
            (*ctx).buffer_pos = (*ctx).buffer_pos.wrapping_add(xlen);
            return RNG_SUCCESS;
        }
        memcpy(
            x.offset(offset as isize) as *mut libc::c_void,
            (&raw mut (*ctx).buffer as *mut libc::c_uchar).offset((*ctx).buffer_pos as isize)
                as *const libc::c_void,
            (16 as size_t).wrapping_sub((*ctx).buffer_pos as size_t),
        );
        xlen = xlen.wrapping_sub((16 as libc::c_ulong).wrapping_sub((*ctx).buffer_pos));
        offset = offset.wrapping_add((16 as libc::c_ulong).wrapping_sub((*ctx).buffer_pos));
        AES256_ECB(
            &raw mut (*ctx).key as *mut libc::c_uchar,
            &raw mut (*ctx).ctr as *mut libc::c_uchar,
            &raw mut (*ctx).buffer as *mut libc::c_uchar,
        );
        (*ctx).buffer_pos = 0 as libc::c_ulong;
        let mut i: libc::c_int = 15 as libc::c_int;
        while i >= 12 as libc::c_int {
            if (*ctx).ctr[i as usize] as libc::c_int == 0xff as libc::c_int {
                (*ctx).ctr[i as usize] = 0 as libc::c_uchar;
                i -= 1;
            } else {
                (*ctx).ctr[i as usize] = (*ctx).ctr[i as usize].wrapping_add(1);
                break;
            }
        }
    }
    return RNG_SUCCESS;
}
unsafe extern "C" fn handleErrors() {
    ERR_print_errors_fp(stderr as *mut FILE);
    abort();
}
#[no_mangle]
pub unsafe extern "C" fn AES256_ECB(
    mut key: *mut libc::c_uchar,
    mut ctr: *mut libc::c_uchar,
    mut buffer: *mut libc::c_uchar,
) {
    let mut ctx: *mut EVP_CIPHER_CTX = std::ptr::null_mut::<EVP_CIPHER_CTX>();
    let mut len: libc::c_int = 0;
    ctx = EVP_CIPHER_CTX_new();
    if ctx.is_null() {
        handleErrors();
    }
    if 1 as libc::c_int
        != EVP_EncryptInit_ex(
            ctx,
            EVP_aes_256_ecb(),
            std::ptr::null_mut::<ENGINE>(),
            key,
            std::ptr::null::<libc::c_uchar>(),
        )
    {
        handleErrors();
    }
    if 1 as libc::c_int
        != EVP_EncryptUpdate(ctx, buffer, &raw mut len, ctr, 16 as libc::c_int)
    {
        handleErrors();
    }
    EVP_CIPHER_CTX_free(ctx);
}
#[no_mangle]
pub unsafe extern "C" fn randombytes_init(
    mut entropy_input: *mut libc::c_uchar,
    mut personalization_string: *mut libc::c_uchar,
) {
    let mut seed_material: [libc::c_uchar; 48] = [0; 48];
    memcpy(
        &raw mut seed_material as *mut libc::c_uchar as *mut libc::c_void,
        entropy_input as *const libc::c_void,
        48 as size_t,
    );
    if !personalization_string.is_null() {
        let mut i: libc::c_int = 0 as libc::c_int;
        while i < 48 as libc::c_int {
            seed_material[i as usize] = (seed_material[i as usize] as libc::c_int
                ^ *personalization_string.offset(i as isize) as libc::c_int)
                as libc::c_uchar;
            i += 1;
        }
    }
    memset(
        &raw mut DRBG_ctx.Key as *mut libc::c_uchar as *mut libc::c_void,
        0 as libc::c_int,
        32 as size_t,
    );
    memset(
        &raw mut DRBG_ctx.V as *mut libc::c_uchar as *mut libc::c_void,
        0 as libc::c_int,
        16 as size_t,
    );
    AES256_CTR_DRBG_Update(
        &raw mut seed_material as *mut libc::c_uchar,
        &raw mut DRBG_ctx.Key as *mut libc::c_uchar,
        &raw mut DRBG_ctx.V as *mut libc::c_uchar,
    );
    DRBG_ctx.reseed_counter = 1 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn randombytes(
    mut x: *mut libc::c_uchar,
    mut xlen: libc::c_ulonglong,
) -> libc::c_int {
    let mut block: [libc::c_uchar; 16] = [0; 16];
    let mut i: libc::c_int = 0 as libc::c_int;
    while xlen > 0 as libc::c_ulonglong {
        let mut j: libc::c_int = 15 as libc::c_int;
        while j >= 0 as libc::c_int {
            if DRBG_ctx.V[j as usize] as libc::c_int == 0xff as libc::c_int {
                DRBG_ctx.V[j as usize] = 0 as libc::c_uchar;
                j -= 1;
            } else {
                DRBG_ctx.V[j as usize] = DRBG_ctx.V[j as usize].wrapping_add(1);
                break;
            }
        }
        AES256_ECB(
            &raw mut DRBG_ctx.Key as *mut libc::c_uchar,
            &raw mut DRBG_ctx.V as *mut libc::c_uchar,
            &raw mut block as *mut libc::c_uchar,
        );
        if xlen > 15 as libc::c_ulonglong {
            memcpy(
                x.offset(i as isize) as *mut libc::c_void,
                &raw mut block as *mut libc::c_uchar as *const libc::c_void,
                16 as size_t,
            );
            i += 16 as libc::c_int;
            xlen = xlen.wrapping_sub(16 as libc::c_ulonglong);
        } else {
            memcpy(
                x.offset(i as isize) as *mut libc::c_void,
                &raw mut block as *mut libc::c_uchar as *const libc::c_void,
                xlen as size_t,
            );
            xlen = 0 as libc::c_ulonglong;
        }
    }
    AES256_CTR_DRBG_Update(
        std::ptr::null_mut::<libc::c_uchar>(),
        &raw mut DRBG_ctx.Key as *mut libc::c_uchar,
        &raw mut DRBG_ctx.V as *mut libc::c_uchar,
    );
    DRBG_ctx.reseed_counter += 1;
    return RNG_SUCCESS;
}
#[no_mangle]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    mut provided_data: *mut libc::c_uchar,
    mut Key: *mut libc::c_uchar,
    mut V: *mut libc::c_uchar,
) {
    let mut temp: [libc::c_uchar; 48] = [0; 48];
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < 3 as libc::c_int {
        let mut j: libc::c_int = 15 as libc::c_int;
        while j >= 0 as libc::c_int {
            if *V.offset(j as isize) as libc::c_int == 0xff as libc::c_int {
                *V.offset(j as isize) = 0 as libc::c_uchar;
                j -= 1;
            } else {
                let ref mut fresh0 = *V.offset(j as isize);
                *fresh0 = (*fresh0).wrapping_add(1);
                break;
            }
        }
        AES256_ECB(
            Key,
            V,
            (&raw mut temp as *mut libc::c_uchar)
                .offset((16 as libc::c_int * i) as isize),
        );
        i += 1;
    }
    if !provided_data.is_null() {
        let mut i_0: libc::c_int = 0 as libc::c_int;
        while i_0 < 48 as libc::c_int {
            temp[i_0 as usize] = (temp[i_0 as usize] as libc::c_int
                ^ *provided_data.offset(i_0 as isize) as libc::c_int)
                as libc::c_uchar;
            i_0 += 1;
        }
    }
    memcpy(
        Key as *mut libc::c_void,
        &raw mut temp as *mut libc::c_uchar as *const libc::c_void,
        32 as size_t,
    );
    memcpy(
        V as *mut libc::c_void,
        (&raw mut temp as *mut libc::c_uchar).offset(32 as libc::c_int as isize)
            as *const libc::c_void,
        16 as size_t,
    );
}
