extern "C" {
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
    fn SPX_haraka_S(
        out: *mut libc::c_uchar,
        outlen: libc::c_ulonglong,
        in_0: *const libc::c_uchar,
        inlen: libc::c_ulonglong,
        ctx: *const spx_ctx,
    );
    fn SPX_haraka512(
        out: *mut libc::c_uchar,
        in_0: *const libc::c_uchar,
        ctx: *const spx_ctx,
    );
    fn SPX_haraka256(
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
pub const SPX_ADDR_BYTES: libc::c_int = 32 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn SPX_thash(
    mut out: *mut libc::c_uchar,
    mut in_0: *const libc::c_uchar,
    mut inblocks: libc::c_uint,
    mut ctx: *const spx_ctx,
    mut addr: *mut uint32_t,
) {
    let vla = (32 as libc::c_uint)
        .wrapping_add(inblocks.wrapping_mul(16 as libc::c_uint)) as usize;
    let mut buf: Vec<uint8_t> = ::std::vec::from_elem(0, vla);
    let vla_0 = inblocks.wrapping_mul(16 as libc::c_uint) as usize;
    let mut bitmask: Vec<uint8_t> = ::std::vec::from_elem(0, vla_0);
    let mut outbuf: [libc::c_uchar; 32] = [0; 32];
    let mut buf_tmp: [libc::c_uchar; 64] = [0; 64];
    let mut i: libc::c_uint = 0;
    if inblocks == 1 as libc::c_uint {
        memset(
            &raw mut buf_tmp as *mut libc::c_uchar as *mut libc::c_void,
            0 as libc::c_int,
            64 as size_t,
        );
        memcpy(
            &raw mut buf_tmp as *mut libc::c_uchar as *mut libc::c_void,
            addr as *const libc::c_void,
            32 as size_t,
        );
        SPX_haraka256(
            &raw mut outbuf as *mut libc::c_uchar,
            &raw mut buf_tmp as *mut libc::c_uchar,
            ctx,
        );
        i = 0 as libc::c_uint;
        while i < inblocks.wrapping_mul(SPX_N as libc::c_uint) {
            buf_tmp[(SPX_ADDR_BYTES as libc::c_uint).wrapping_add(i) as usize] =
                (*in_0.offset(i as isize) as libc::c_int
                    ^ outbuf[i as usize] as libc::c_int)
                    as libc::c_uchar;
            i = i.wrapping_add(1);
        }
        SPX_haraka512(
            &raw mut outbuf as *mut libc::c_uchar,
            &raw mut buf_tmp as *mut libc::c_uchar,
            ctx,
        );
        memcpy(
            out as *mut libc::c_void,
            &raw mut outbuf as *mut libc::c_uchar as *const libc::c_void,
            SPX_N as size_t,
        );
    } else {
        memcpy(
            buf.as_mut_ptr() as *mut libc::c_void,
            addr as *const libc::c_void,
            32 as size_t,
        );
        SPX_haraka_S(
            bitmask.as_mut_ptr(),
            inblocks.wrapping_mul(SPX_N as libc::c_uint) as libc::c_ulonglong,
            buf.as_mut_ptr(),
            SPX_ADDR_BYTES as libc::c_ulonglong,
            ctx,
        );
        i = 0 as libc::c_uint;
        while i < inblocks.wrapping_mul(SPX_N as libc::c_uint) {
            *buf.as_mut_ptr()
                .offset((SPX_ADDR_BYTES as libc::c_uint).wrapping_add(i) as isize) =
                (*in_0.offset(i as isize) as libc::c_int
                    ^ *bitmask.as_mut_ptr().offset(i as isize) as libc::c_int)
                    as uint8_t;
            i = i.wrapping_add(1);
        }
        SPX_haraka_S(
            out,
            SPX_N as libc::c_ulonglong,
            buf.as_mut_ptr(),
            (SPX_ADDR_BYTES as libc::c_uint)
                .wrapping_add(inblocks.wrapping_mul(SPX_N as libc::c_uint))
                as libc::c_ulonglong,
            ctx,
        );
    };
}
