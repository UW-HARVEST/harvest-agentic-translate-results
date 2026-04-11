extern "C" {
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn SPX_set_type(addr: *mut uint32_t, type_0: uint32_t);
    fn SPX_set_keypair_addr(addr: *mut uint32_t, keypair: uint32_t);
    fn SPX_set_chain_addr(addr: *mut uint32_t, chain: uint32_t);
    fn SPX_set_hash_addr(addr: *mut uint32_t, hash: uint32_t);
    fn SPX_prf_addr(out: *mut libc::c_uchar, ctx: *const spx_ctx, addr: *const uint32_t);
    fn SPX_thash(
        out: *mut libc::c_uchar,
        in_0: *const libc::c_uchar,
        inblocks: libc::c_uint,
        ctx: *const spx_ctx,
        addr: *mut uint32_t,
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct leaf_info_x1 {
    pub wots_sig: *mut libc::c_uchar,
    pub wots_sign_leaf: uint32_t,
    pub wots_steps: *mut uint32_t,
    pub leaf_addr: [uint32_t; 8],
    pub pk_addr: [uint32_t; 8],
}
pub const SPX_N: libc::c_int = 16 as libc::c_int;
pub const SPX_WOTS_W: libc::c_int = 16 as libc::c_int;
pub const SPX_WOTS_LOGW: libc::c_int = 4 as libc::c_int;
pub const SPX_WOTS_LEN1: libc::c_int = 8 as libc::c_int * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: libc::c_int = 3 as libc::c_int;
pub const SPX_WOTS_LEN: libc::c_int = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_ADDR_TYPE_WOTS: libc::c_int = 0 as libc::c_int;
pub const SPX_ADDR_TYPE_WOTSPRF: libc::c_int = 5 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn SPX_wots_gen_leafx1(
    mut dest: *mut libc::c_uchar,
    mut ctx: *const spx_ctx,
    mut leaf_idx: uint32_t,
    mut v_info: *mut leaf_info_x1,
) {
    let mut info: *mut leaf_info_x1 = v_info as *mut leaf_info_x1;
    let mut leaf_addr: *mut uint32_t = &raw mut (*info).leaf_addr as *mut uint32_t;
    let mut pk_addr: *mut uint32_t = &raw mut (*info).pk_addr as *mut uint32_t;
    let mut i: libc::c_uint = 0;
    let mut k: libc::c_uint = 0;
    let mut pk_buffer: [libc::c_uchar; 560] = [0; 560];
    let mut buffer: *mut libc::c_uchar = std::ptr::null_mut::<libc::c_uchar>();
    let mut wots_k_mask: uint32_t = 0;
    if leaf_idx == (*info).wots_sign_leaf {
        wots_k_mask = 0 as uint32_t;
    } else {
        wots_k_mask = !(0 as libc::c_int) as uint32_t;
    }
    SPX_set_keypair_addr(leaf_addr as *mut uint32_t, leaf_idx);
    SPX_set_keypair_addr(pk_addr as *mut uint32_t, leaf_idx);
    i = 0 as libc::c_uint;
    buffer = &raw mut pk_buffer as *mut libc::c_uchar;
    while i < SPX_WOTS_LEN as libc::c_uint {
        let mut wots_k: uint32_t = *(*info).wots_steps.offset(i as isize) | wots_k_mask;
        SPX_set_chain_addr(leaf_addr as *mut uint32_t, i as uint32_t);
        SPX_set_hash_addr(leaf_addr as *mut uint32_t, 0 as uint32_t);
        SPX_set_type(
            leaf_addr as *mut uint32_t,
            SPX_ADDR_TYPE_WOTSPRF as uint32_t,
        );
        SPX_prf_addr(buffer, ctx, leaf_addr as *const uint32_t);
        SPX_set_type(leaf_addr as *mut uint32_t, SPX_ADDR_TYPE_WOTS as uint32_t);
        k = 0 as libc::c_uint;
        loop {
            if k as uint32_t == wots_k {
                memcpy(
                    (*info)
                        .wots_sig
                        .offset(i.wrapping_mul(SPX_N as libc::c_uint) as isize)
                        as *mut libc::c_void,
                    buffer as *const libc::c_void,
                    SPX_N as size_t,
                );
            }
            if k == (SPX_WOTS_W - 1 as libc::c_int) as libc::c_uint {
                break;
            }
            SPX_set_hash_addr(leaf_addr as *mut uint32_t, k as uint32_t);
            SPX_thash(
                buffer,
                buffer,
                1 as libc::c_uint,
                ctx,
                leaf_addr as *mut uint32_t,
            );
            k = k.wrapping_add(1);
        }
        i = i.wrapping_add(1);
        buffer = buffer.offset(SPX_N as isize);
    }
    SPX_thash(
        dest,
        &raw mut pk_buffer as *mut libc::c_uchar,
        SPX_WOTS_LEN as libc::c_uint,
        ctx,
        pk_addr as *mut uint32_t,
    );
}
