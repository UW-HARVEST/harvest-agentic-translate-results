extern "C" {
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn SPX_bytes_to_ull(
        in_0: *const libc::c_uchar,
        inlen: libc::c_uint,
    ) -> libc::c_ulonglong;
    fn SPX_tweak_constants(ctx: *mut spx_ctx);
    fn SPX_haraka_S_inc_init(s_inc: *mut uint8_t);
    fn SPX_haraka_S_inc_absorb(
        s_inc: *mut uint8_t,
        m: *const uint8_t,
        mlen: size_t,
        ctx: *const spx_ctx,
    );
    fn SPX_haraka_S_inc_finalize(s_inc: *mut uint8_t);
    fn SPX_haraka_S_inc_squeeze(
        out: *mut uint8_t,
        outlen: size_t,
        s_inc: *mut uint8_t,
        ctx: *const spx_ctx,
    );
    fn SPX_haraka512(
        out: *mut libc::c_uchar,
        in_0: *const libc::c_uchar,
        ctx: *const spx_ctx,
    );
}
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct spx_ctx {
    pub pub_seed: [uint8_t; 16],
    pub sk_seed: [uint8_t; 16],
    pub tweaked512_rc64: [[uint64_t; 8]; 10],
    pub tweaked256_rc32: [[uint32_t; 8]; 10],
}
pub const SPX_N: libc::c_int = 16 as libc::c_int;
pub const SPX_FULL_HEIGHT: libc::c_int = 63 as libc::c_int;
pub const SPX_D: libc::c_int = 7 as libc::c_int;
pub const SPX_FORS_HEIGHT: libc::c_int = 12 as libc::c_int;
pub const SPX_FORS_TREES: libc::c_int = 14 as libc::c_int;
pub const SPX_ADDR_BYTES: libc::c_int = 32 as libc::c_int;
pub const SPX_TREE_HEIGHT: libc::c_int = SPX_FULL_HEIGHT / SPX_D;
pub const SPX_FORS_MSG_BYTES: libc::c_int =
    (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7 as libc::c_int) / 8 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn SPX_initialize_hash_function(mut ctx: *mut spx_ctx) {
    SPX_tweak_constants(ctx);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_prf_addr(
    mut out: *mut libc::c_uchar,
    mut ctx: *const spx_ctx,
    mut addr: *const uint32_t,
) {
    let mut outbuf: [libc::c_uchar; 32] = [0; 32];
    let mut buf: [libc::c_uchar; 64] = [
        0 as libc::c_int as libc::c_uchar,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    memcpy(
        &raw mut buf as *mut libc::c_uchar as *mut libc::c_void,
        addr as *const libc::c_void,
        SPX_ADDR_BYTES as size_t,
    );
    memcpy(
        (&raw mut buf as *mut libc::c_uchar).offset(SPX_ADDR_BYTES as isize)
            as *mut libc::c_void,
        &raw const (*ctx).sk_seed as *const uint8_t as *const libc::c_void,
        SPX_N as size_t,
    );
    SPX_haraka512(
        &raw mut outbuf as *mut libc::c_uchar,
        &raw mut buf as *mut libc::c_uchar,
        ctx,
    );
    memcpy(
        out as *mut libc::c_void,
        &raw mut outbuf as *mut libc::c_uchar as *const libc::c_void,
        SPX_N as size_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_gen_message_random(
    mut R: *mut libc::c_uchar,
    mut sk_prf: *const libc::c_uchar,
    mut optrand: *const libc::c_uchar,
    mut m: *const libc::c_uchar,
    mut mlen: libc::c_ulonglong,
    mut ctx: *const spx_ctx,
) {
    let mut s_inc: [uint8_t; 65] = [0; 65];
    SPX_haraka_S_inc_init(&raw mut s_inc as *mut uint8_t);
    SPX_haraka_S_inc_absorb(
        &raw mut s_inc as *mut uint8_t,
        sk_prf as *const uint8_t,
        SPX_N as size_t,
        ctx,
    );
    SPX_haraka_S_inc_absorb(
        &raw mut s_inc as *mut uint8_t,
        optrand as *const uint8_t,
        SPX_N as size_t,
        ctx,
    );
    SPX_haraka_S_inc_absorb(
        &raw mut s_inc as *mut uint8_t,
        m as *const uint8_t,
        mlen as size_t,
        ctx,
    );
    SPX_haraka_S_inc_finalize(&raw mut s_inc as *mut uint8_t);
    SPX_haraka_S_inc_squeeze(
        R as *mut uint8_t,
        SPX_N as size_t,
        &raw mut s_inc as *mut uint8_t,
        ctx,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_hash_message(
    mut digest: *mut libc::c_uchar,
    mut tree: *mut uint64_t,
    mut leaf_idx: *mut uint32_t,
    mut R: *const libc::c_uchar,
    mut pk: *const libc::c_uchar,
    mut m: *const libc::c_uchar,
    mut mlen: libc::c_ulonglong,
    mut ctx: *const spx_ctx,
) {
    let mut buf: [libc::c_uchar; 30] = [0; 30];
    let mut bufp: *mut libc::c_uchar = &raw mut buf as *mut libc::c_uchar;
    let mut s_inc: [uint8_t; 65] = [0; 65];
    SPX_haraka_S_inc_init(&raw mut s_inc as *mut uint8_t);
    SPX_haraka_S_inc_absorb(
        &raw mut s_inc as *mut uint8_t,
        R as *const uint8_t,
        SPX_N as size_t,
        ctx,
    );
    SPX_haraka_S_inc_absorb(
        &raw mut s_inc as *mut uint8_t,
        pk.offset(SPX_N as isize),
        SPX_N as size_t,
        ctx,
    );
    SPX_haraka_S_inc_absorb(
        &raw mut s_inc as *mut uint8_t,
        m as *const uint8_t,
        mlen as size_t,
        ctx,
    );
    SPX_haraka_S_inc_finalize(&raw mut s_inc as *mut uint8_t);
    SPX_haraka_S_inc_squeeze(
        &raw mut buf as *mut uint8_t,
        SPX_DGST_BYTES as size_t,
        &raw mut s_inc as *mut uint8_t,
        ctx,
    );
    memcpy(
        digest as *mut libc::c_void,
        bufp as *const libc::c_void,
        SPX_FORS_MSG_BYTES as size_t,
    );
    bufp = bufp.offset(SPX_FORS_MSG_BYTES as isize);
    if SPX_D == 1 as libc::c_int {
        *tree = 0 as uint64_t;
    } else {
        *tree = SPX_bytes_to_ull(bufp, SPX_TREE_BYTES as libc::c_uint) as uint64_t;
        *tree = (*tree as libc::c_ulong
            & (!(0 as libc::c_int as uint64_t) >> 64 as libc::c_int - SPX_TREE_BITS)
                as libc::c_ulong) as uint64_t;
    }
    bufp = bufp.offset(SPX_TREE_BYTES as isize);
    *leaf_idx = SPX_bytes_to_ull(bufp, SPX_LEAF_BYTES as libc::c_uint) as uint32_t;
    *leaf_idx = (*leaf_idx as libc::c_uint
        & (!(0 as libc::c_int as uint32_t) >> 32 as libc::c_int - SPX_LEAF_BITS)
            as libc::c_uint) as uint32_t;
}
pub const SPX_TREE_BITS: libc::c_int = SPX_TREE_HEIGHT * (SPX_D - 1 as libc::c_int);
pub const SPX_TREE_BYTES: libc::c_int =
    (SPX_TREE_BITS + 7 as libc::c_int) / 8 as libc::c_int;
pub const SPX_LEAF_BITS: libc::c_int = SPX_TREE_HEIGHT;
pub const SPX_LEAF_BYTES: libc::c_int =
    (SPX_LEAF_BITS + 7 as libc::c_int) / 8 as libc::c_int;
pub const SPX_DGST_BYTES: libc::c_int = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;
