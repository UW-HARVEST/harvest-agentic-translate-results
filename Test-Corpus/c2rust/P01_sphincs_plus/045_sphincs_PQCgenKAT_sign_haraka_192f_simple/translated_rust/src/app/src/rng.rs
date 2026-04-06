extern "C" {
    pub type evp_cipher_st;
    pub type evp_cipher_ctx_st;
    pub type engine_st;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    static mut stderr: *mut _IO_FILE;
    fn abort() -> !;
    fn EVP_EncryptInit_ex(
        ctx: *mut EVP_CIPHER_CTX,
        cipher: *const EVP_CIPHER,
        impl_0: *mut ENGINE,
        key: *const ::core::ffi::c_uchar,
        iv: *const ::core::ffi::c_uchar,
    ) -> ::core::ffi::c_int;
    fn EVP_EncryptUpdate(
        ctx: *mut EVP_CIPHER_CTX,
        out: *mut ::core::ffi::c_uchar,
        outl: *mut ::core::ffi::c_int,
        in_0: *const ::core::ffi::c_uchar,
        inl: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn EVP_CIPHER_CTX_new() -> *mut EVP_CIPHER_CTX;
    fn EVP_CIPHER_CTX_free(c: *mut EVP_CIPHER_CTX);
    fn EVP_aes_256_ecb() -> *const EVP_CIPHER;
    fn ERR_print_errors_fp(fp: *mut FILE);
}
pub type size_t = usize;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut ::core::ffi::c_void,
    pub __pad2: *mut ::core::ffi::c_void,
    pub __pad3: *mut ::core::ffi::c_void,
    pub __pad4: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
pub type EVP_CIPHER = evp_cipher_st;
pub type EVP_CIPHER_CTX = evp_cipher_ctx_st;
pub type ENGINE = engine_st;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct AES_XOF_struct {
    pub buffer: [::core::ffi::c_uchar; 16],
    pub buffer_pos: ::core::ffi::c_ulong,
    pub length_remaining: ::core::ffi::c_ulong,
    pub key: [::core::ffi::c_uchar; 32],
    pub ctr: [::core::ffi::c_uchar; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct AES256_CTR_DRBG_struct {
    pub Key: [::core::ffi::c_uchar; 32],
    pub V: [::core::ffi::c_uchar; 16],
    pub reseed_counter: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const RNG_SUCCESS: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const RNG_BAD_MAXLEN: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const RNG_BAD_OUTBUF: ::core::ffi::c_int = -(2 as ::core::ffi::c_int);
pub const RNG_BAD_REQ_LEN: ::core::ffi::c_int = -(3 as ::core::ffi::c_int);
#[no_mangle]
pub static mut DRBG_ctx: AES256_CTR_DRBG_struct = AES256_CTR_DRBG_struct {
    Key: [0; 32],
    V: [0; 16],
    reseed_counter: 0,
};
#[no_mangle]
pub unsafe extern "C" fn seedexpander_init(
    mut ctx: *mut AES_XOF_struct,
    mut seed: *mut ::core::ffi::c_uchar,
    mut diversifier: *mut ::core::ffi::c_uchar,
    mut maxlen: ::core::ffi::c_ulong,
) -> ::core::ffi::c_int {
    if maxlen >= 0x100000000 as ::core::ffi::c_long as ::core::ffi::c_ulong {
        return RNG_BAD_MAXLEN;
    }
    (*ctx).length_remaining = maxlen;
    memcpy(
        &raw mut (*ctx).key as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
        seed as *const ::core::ffi::c_void,
        32 as size_t,
    );
    memcpy(
        &raw mut (*ctx).ctr as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
        diversifier as *const ::core::ffi::c_void,
        8 as size_t,
    );
    (*ctx).ctr[11 as ::core::ffi::c_int as usize] =
        maxlen.wrapping_rem(256 as ::core::ffi::c_ulong) as ::core::ffi::c_uchar;
    maxlen >>= 8 as ::core::ffi::c_int;
    (*ctx).ctr[10 as ::core::ffi::c_int as usize] =
        maxlen.wrapping_rem(256 as ::core::ffi::c_ulong) as ::core::ffi::c_uchar;
    maxlen >>= 8 as ::core::ffi::c_int;
    (*ctx).ctr[9 as ::core::ffi::c_int as usize] =
        maxlen.wrapping_rem(256 as ::core::ffi::c_ulong) as ::core::ffi::c_uchar;
    maxlen >>= 8 as ::core::ffi::c_int;
    (*ctx).ctr[8 as ::core::ffi::c_int as usize] =
        maxlen.wrapping_rem(256 as ::core::ffi::c_ulong) as ::core::ffi::c_uchar;
    memset(
        (&raw mut (*ctx).ctr as *mut ::core::ffi::c_uchar).offset(12 as ::core::ffi::c_int as isize)
            as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        4 as size_t,
    );
    (*ctx).buffer_pos = 16 as ::core::ffi::c_ulong;
    memset(
        &raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        16 as size_t,
    );
    return RNG_SUCCESS;
}
#[no_mangle]
pub unsafe extern "C" fn seedexpander(
    mut ctx: *mut AES_XOF_struct,
    mut x: *mut ::core::ffi::c_uchar,
    mut xlen: ::core::ffi::c_ulong,
) -> ::core::ffi::c_int {
    let mut offset: ::core::ffi::c_ulong = 0;
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    if xlen >= (*ctx).length_remaining {
        return RNG_BAD_REQ_LEN;
    }
    (*ctx).length_remaining = (*ctx).length_remaining.wrapping_sub(xlen);
    offset = 0 as ::core::ffi::c_ulong;
    while xlen > 0 as ::core::ffi::c_ulong {
        if xlen <= (16 as ::core::ffi::c_ulong).wrapping_sub((*ctx).buffer_pos) {
            memcpy(
                x.offset(offset as isize) as *mut ::core::ffi::c_void,
                (&raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar)
                    .offset((*ctx).buffer_pos as isize)
                    as *const ::core::ffi::c_void,
                xlen as size_t,
            );
            (*ctx).buffer_pos = (*ctx).buffer_pos.wrapping_add(xlen);
            return RNG_SUCCESS;
        }
        memcpy(
            x.offset(offset as isize) as *mut ::core::ffi::c_void,
            (&raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar).offset((*ctx).buffer_pos as isize)
                as *const ::core::ffi::c_void,
            (16 as size_t).wrapping_sub((*ctx).buffer_pos as size_t),
        );
        xlen = xlen.wrapping_sub((16 as ::core::ffi::c_ulong).wrapping_sub((*ctx).buffer_pos));
        offset = offset.wrapping_add((16 as ::core::ffi::c_ulong).wrapping_sub((*ctx).buffer_pos));
        AES256_ECB(
            &raw mut (*ctx).key as *mut ::core::ffi::c_uchar,
            &raw mut (*ctx).ctr as *mut ::core::ffi::c_uchar,
            &raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar,
        );
        (*ctx).buffer_pos = 0 as ::core::ffi::c_ulong;
        let mut i: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
        while i >= 12 as ::core::ffi::c_int {
            if (*ctx).ctr[i as usize] as ::core::ffi::c_int == 0xff as ::core::ffi::c_int {
                (*ctx).ctr[i as usize] = 0 as ::core::ffi::c_uchar;
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
    mut key: *mut ::core::ffi::c_uchar,
    mut ctr: *mut ::core::ffi::c_uchar,
    mut buffer: *mut ::core::ffi::c_uchar,
) {
    let mut ctx: *mut EVP_CIPHER_CTX = ::core::ptr::null_mut::<EVP_CIPHER_CTX>();
    let mut len: ::core::ffi::c_int = 0;
    ctx = EVP_CIPHER_CTX_new();
    if ctx.is_null() {
        handleErrors();
    }
    if 1 as ::core::ffi::c_int
        != EVP_EncryptInit_ex(
            ctx,
            EVP_aes_256_ecb(),
            ::core::ptr::null_mut::<ENGINE>(),
            key,
            ::core::ptr::null::<::core::ffi::c_uchar>(),
        )
    {
        handleErrors();
    }
    if 1 as ::core::ffi::c_int
        != EVP_EncryptUpdate(ctx, buffer, &raw mut len, ctr, 16 as ::core::ffi::c_int)
    {
        handleErrors();
    }
    EVP_CIPHER_CTX_free(ctx);
}
#[no_mangle]
pub unsafe extern "C" fn randombytes_init(
    mut entropy_input: *mut ::core::ffi::c_uchar,
    mut personalization_string: *mut ::core::ffi::c_uchar,
) {
    let mut seed_material: [::core::ffi::c_uchar; 48] = [0; 48];
    memcpy(
        &raw mut seed_material as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
        entropy_input as *const ::core::ffi::c_void,
        48 as size_t,
    );
    if !personalization_string.is_null() {
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < 48 as ::core::ffi::c_int {
            seed_material[i as usize] = (seed_material[i as usize] as ::core::ffi::c_int
                ^ *personalization_string.offset(i as isize) as ::core::ffi::c_int)
                as ::core::ffi::c_uchar;
            i += 1;
        }
    }
    memset(
        &raw mut DRBG_ctx.Key as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        32 as size_t,
    );
    memset(
        &raw mut DRBG_ctx.V as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        16 as size_t,
    );
    AES256_CTR_DRBG_Update(
        &raw mut seed_material as *mut ::core::ffi::c_uchar,
        &raw mut DRBG_ctx.Key as *mut ::core::ffi::c_uchar,
        &raw mut DRBG_ctx.V as *mut ::core::ffi::c_uchar,
    );
    DRBG_ctx.reseed_counter = 1 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn randombytes(
    mut x: *mut ::core::ffi::c_uchar,
    mut xlen: ::core::ffi::c_ulonglong,
) -> ::core::ffi::c_int {
    let mut block: [::core::ffi::c_uchar; 16] = [0; 16];
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while xlen > 0 as ::core::ffi::c_ulonglong {
        let mut j: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
        while j >= 0 as ::core::ffi::c_int {
            if DRBG_ctx.V[j as usize] as ::core::ffi::c_int == 0xff as ::core::ffi::c_int {
                DRBG_ctx.V[j as usize] = 0 as ::core::ffi::c_uchar;
                j -= 1;
            } else {
                DRBG_ctx.V[j as usize] = DRBG_ctx.V[j as usize].wrapping_add(1);
                break;
            }
        }
        AES256_ECB(
            &raw mut DRBG_ctx.Key as *mut ::core::ffi::c_uchar,
            &raw mut DRBG_ctx.V as *mut ::core::ffi::c_uchar,
            &raw mut block as *mut ::core::ffi::c_uchar,
        );
        if xlen > 15 as ::core::ffi::c_ulonglong {
            memcpy(
                x.offset(i as isize) as *mut ::core::ffi::c_void,
                &raw mut block as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
                16 as size_t,
            );
            i += 16 as ::core::ffi::c_int;
            xlen = xlen.wrapping_sub(16 as ::core::ffi::c_ulonglong);
        } else {
            memcpy(
                x.offset(i as isize) as *mut ::core::ffi::c_void,
                &raw mut block as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
                xlen as size_t,
            );
            xlen = 0 as ::core::ffi::c_ulonglong;
        }
    }
    AES256_CTR_DRBG_Update(
        ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        &raw mut DRBG_ctx.Key as *mut ::core::ffi::c_uchar,
        &raw mut DRBG_ctx.V as *mut ::core::ffi::c_uchar,
    );
    DRBG_ctx.reseed_counter += 1;
    return RNG_SUCCESS;
}
#[no_mangle]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    mut provided_data: *mut ::core::ffi::c_uchar,
    mut Key: *mut ::core::ffi::c_uchar,
    mut V: *mut ::core::ffi::c_uchar,
) {
    let mut temp: [::core::ffi::c_uchar; 48] = [0; 48];
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 3 as ::core::ffi::c_int {
        let mut j: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
        while j >= 0 as ::core::ffi::c_int {
            if *V.offset(j as isize) as ::core::ffi::c_int == 0xff as ::core::ffi::c_int {
                *V.offset(j as isize) = 0 as ::core::ffi::c_uchar;
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
            (&raw mut temp as *mut ::core::ffi::c_uchar)
                .offset((16 as ::core::ffi::c_int * i) as isize),
        );
        i += 1;
    }
    if !provided_data.is_null() {
        let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i_0 < 48 as ::core::ffi::c_int {
            temp[i_0 as usize] = (temp[i_0 as usize] as ::core::ffi::c_int
                ^ *provided_data.offset(i_0 as isize) as ::core::ffi::c_int)
                as ::core::ffi::c_uchar;
            i_0 += 1;
        }
    }
    memcpy(
        Key as *mut ::core::ffi::c_void,
        &raw mut temp as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
        32 as size_t,
    );
    memcpy(
        V as *mut ::core::ffi::c_void,
        (&raw mut temp as *mut ::core::ffi::c_uchar).offset(32 as ::core::ffi::c_int as isize)
            as *const ::core::ffi::c_void,
        16 as size_t,
    );
}
