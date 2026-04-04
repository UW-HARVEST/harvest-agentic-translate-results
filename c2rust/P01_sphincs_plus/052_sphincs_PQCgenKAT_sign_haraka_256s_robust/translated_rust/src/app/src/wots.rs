extern "C" {
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn SPX_set_chain_addr(addr: *mut uint32_t, chain: uint32_t);
    fn SPX_set_hash_addr(addr: *mut uint32_t, hash: uint32_t);
    fn SPX_thash(
        out: *mut ::core::ffi::c_uchar,
        in_0: *const ::core::ffi::c_uchar,
        inblocks: ::core::ffi::c_uint,
        ctx: *const spx_ctx,
        addr: *mut uint32_t,
    );
    fn SPX_ull_to_bytes(
        out: *mut ::core::ffi::c_uchar,
        outlen: ::core::ffi::c_uint,
        in_0: ::core::ffi::c_ulonglong,
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
pub const SPX_N: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const SPX_WOTS_W: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const SPX_WOTS_LOGW: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const SPX_WOTS_LEN1: ::core::ffi::c_int = 8 as ::core::ffi::c_int * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const SPX_WOTS_LEN: ::core::ffi::c_int = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
unsafe extern "C" fn gen_chain(
    mut out: *mut ::core::ffi::c_uchar,
    mut in_0: *const ::core::ffi::c_uchar,
    mut start: ::core::ffi::c_uint,
    mut steps: ::core::ffi::c_uint,
    mut ctx: *const spx_ctx,
    mut addr: *mut uint32_t,
) {
    let mut i: uint32_t = 0;
    memcpy(
        out as *mut ::core::ffi::c_void,
        in_0 as *const ::core::ffi::c_void,
        SPX_N as size_t,
    );
    i = start as uint32_t;
    while i < (start as uint32_t).wrapping_add(steps as uint32_t) && i < SPX_WOTS_W as uint32_t {
        SPX_set_hash_addr(addr, i);
        SPX_thash(out, out, 1 as ::core::ffi::c_uint, ctx, addr);
        i = i.wrapping_add(1);
    }
}
unsafe extern "C" fn base_w(
    mut output: *mut ::core::ffi::c_uint,
    out_len: ::core::ffi::c_int,
    mut input: *const ::core::ffi::c_uchar,
) {
    let mut in_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut out: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut total: ::core::ffi::c_uchar = 0;
    let mut bits: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut consumed: ::core::ffi::c_int = 0;
    consumed = 0 as ::core::ffi::c_int;
    while consumed < out_len {
        if bits == 0 as ::core::ffi::c_int {
            total = *input.offset(in_0 as isize);
            in_0 += 1;
            bits += 8 as ::core::ffi::c_int;
        }
        bits -= SPX_WOTS_LOGW;
        *output.offset(out as isize) = (total as ::core::ffi::c_int >> bits
            & SPX_WOTS_W - 1 as ::core::ffi::c_int)
            as ::core::ffi::c_uint;
        out += 1;
        consumed += 1;
    }
}
unsafe extern "C" fn wots_checksum(
    mut csum_base_w: *mut ::core::ffi::c_uint,
    mut msg_base_w: *const ::core::ffi::c_uint,
) {
    let mut csum: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut csum_bytes: [::core::ffi::c_uchar; 2] = [0; 2];
    let mut i: ::core::ffi::c_uint = 0;
    i = 0 as ::core::ffi::c_uint;
    while i < SPX_WOTS_LEN1 as ::core::ffi::c_uint {
        csum = csum.wrapping_add(
            ((SPX_WOTS_W - 1 as ::core::ffi::c_int) as ::core::ffi::c_uint)
                .wrapping_sub(*msg_base_w.offset(i as isize)),
        );
        i = i.wrapping_add(1);
    }
    csum = csum
        << (8 as ::core::ffi::c_int - SPX_WOTS_LEN2 * SPX_WOTS_LOGW % 8 as ::core::ffi::c_int)
            % 8 as ::core::ffi::c_int;
    SPX_ull_to_bytes(
        &raw mut csum_bytes as *mut ::core::ffi::c_uchar,
        ::core::mem::size_of::<[::core::ffi::c_uchar; 2]>() as ::core::ffi::c_uint,
        csum as ::core::ffi::c_ulonglong,
    );
    base_w(
        csum_base_w,
        SPX_WOTS_LEN2,
        &raw mut csum_bytes as *mut ::core::ffi::c_uchar,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_chain_lengths(
    mut lengths: *mut ::core::ffi::c_uint,
    mut msg: *const ::core::ffi::c_uchar,
) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    wots_checksum(lengths.offset(SPX_WOTS_LEN1 as isize), lengths);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_wots_pk_from_sig(
    mut pk: *mut ::core::ffi::c_uchar,
    mut sig: *const ::core::ffi::c_uchar,
    mut msg: *const ::core::ffi::c_uchar,
    mut ctx: *const spx_ctx,
    mut addr: *mut uint32_t,
) {
    let mut lengths: [::core::ffi::c_uint; 35] = [0; 35];
    let mut i: uint32_t = 0;
    SPX_chain_lengths(&raw mut lengths as *mut ::core::ffi::c_uint, msg);
    i = 0 as uint32_t;
    while i < SPX_WOTS_LEN as uint32_t {
        SPX_set_chain_addr(addr, i);
        gen_chain(
            pk.offset(i.wrapping_mul(SPX_N as uint32_t) as isize),
            sig.offset(i.wrapping_mul(SPX_N as uint32_t) as isize),
            lengths[i as usize],
            ((SPX_WOTS_W - 1 as ::core::ffi::c_int) as ::core::ffi::c_uint)
                .wrapping_sub(lengths[i as usize]),
            ctx,
            addr,
        );
        i = i.wrapping_add(1);
    }
}
