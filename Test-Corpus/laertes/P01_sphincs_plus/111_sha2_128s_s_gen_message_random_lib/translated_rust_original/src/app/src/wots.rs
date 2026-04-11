extern "C" {
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn SPX_set_chain_addr(addr: *mut uint32_t, chain: uint32_t);
    fn SPX_set_hash_addr(addr: *mut uint32_t, hash: uint32_t);
    fn SPX_thash(
        out: *mut libc::c_uchar,
        in_0: *const libc::c_uchar,
        inblocks: libc::c_uint,
        ctx: *const spx_ctx,
        addr: *mut uint32_t,
    );
    fn SPX_ull_to_bytes(
        out: *mut libc::c_uchar,
        outlen: libc::c_uint,
        in_0: libc::c_ulonglong,
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
pub const SPX_WOTS_W: libc::c_int = 16 as libc::c_int;
pub const SPX_WOTS_LOGW: libc::c_int = 4 as libc::c_int;
pub const SPX_WOTS_LEN1: libc::c_int = 8 as libc::c_int * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: libc::c_int = 3 as libc::c_int;
pub const SPX_WOTS_LEN: libc::c_int = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
unsafe extern "C" fn gen_chain(
    mut out: *mut libc::c_uchar,
    mut in_0: *const libc::c_uchar,
    mut start: libc::c_uint,
    mut steps: libc::c_uint,
    mut ctx: *const spx_ctx,
    mut addr: *mut uint32_t,
) {
    let mut i: uint32_t = 0;
    memcpy(
        out as *mut libc::c_void,
        in_0 as *const libc::c_void,
        SPX_N as size_t,
    );
    i = start as uint32_t;
    while i < (start as uint32_t).wrapping_add(steps as uint32_t) && i < SPX_WOTS_W as uint32_t {
        SPX_set_hash_addr(addr, i);
        SPX_thash(out, out, 1 as libc::c_uint, ctx, addr);
        i = i.wrapping_add(1);
    }
}
unsafe extern "C" fn base_w(
    mut output: *mut libc::c_uint,
    out_len: libc::c_int,
    mut input: *const libc::c_uchar,
) {
    let mut in_0: libc::c_int = 0 as libc::c_int;
    let mut out: libc::c_int = 0 as libc::c_int;
    let mut total: libc::c_uchar = 0;
    let mut bits: libc::c_int = 0 as libc::c_int;
    let mut consumed: libc::c_int = 0;
    consumed = 0 as libc::c_int;
    while consumed < out_len {
        if bits == 0 as libc::c_int {
            total = *input.offset(in_0 as isize);
            in_0 += 1;
            bits += 8 as libc::c_int;
        }
        bits -= SPX_WOTS_LOGW;
        *output.offset(out as isize) = (total as libc::c_int >> bits
            & SPX_WOTS_W - 1 as libc::c_int)
            as libc::c_uint;
        out += 1;
        consumed += 1;
    }
}
unsafe extern "C" fn wots_checksum(
    mut csum_base_w: *mut libc::c_uint,
    mut msg_base_w: *const libc::c_uint,
) {
    let mut csum: libc::c_uint = 0 as libc::c_uint;
    let mut csum_bytes: [libc::c_uchar; 2] = [0; 2];
    let mut i: libc::c_uint = 0;
    i = 0 as libc::c_uint;
    while i < SPX_WOTS_LEN1 as libc::c_uint {
        csum = csum.wrapping_add(
            ((SPX_WOTS_W - 1 as libc::c_int) as libc::c_uint)
                .wrapping_sub(*msg_base_w.offset(i as isize)),
        );
        i = i.wrapping_add(1);
    }
    csum = csum
        << (8 as libc::c_int - SPX_WOTS_LEN2 * SPX_WOTS_LOGW % 8 as libc::c_int)
            % 8 as libc::c_int;
    SPX_ull_to_bytes(
        &raw mut csum_bytes as *mut libc::c_uchar,
        std::mem::size_of::<[libc::c_uchar; 2]>() as libc::c_uint,
        csum as libc::c_ulonglong,
    );
    base_w(
        csum_base_w,
        SPX_WOTS_LEN2,
        &raw mut csum_bytes as *mut libc::c_uchar,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_chain_lengths(
    mut lengths: *mut libc::c_uint,
    mut msg: *const libc::c_uchar,
) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    wots_checksum(lengths.offset(SPX_WOTS_LEN1 as isize), lengths);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    mut pk: *mut libc::c_uchar,
    mut sig: *const libc::c_uchar,
    mut msg: *const libc::c_uchar,
    mut ctx: *const spx_ctx,
    mut addr: *mut uint32_t,
) {
    let mut lengths: [libc::c_uint; 35] = [0; 35];
    let mut i: uint32_t = 0;
    SPX_chain_lengths(&raw mut lengths as *mut libc::c_uint, msg);
    i = 0 as uint32_t;
    while i < SPX_WOTS_LEN as uint32_t {
        SPX_set_chain_addr(addr, i);
        gen_chain(
            pk.offset(i.wrapping_mul(SPX_N as uint32_t) as isize),
            sig.offset(i.wrapping_mul(SPX_N as uint32_t) as isize),
            lengths[i as usize],
            ((SPX_WOTS_W - 1 as libc::c_int) as libc::c_uint)
                .wrapping_sub(lengths[i as usize]),
            ctx,
            addr,
        );
        i = i.wrapping_add(1);
    }
}
