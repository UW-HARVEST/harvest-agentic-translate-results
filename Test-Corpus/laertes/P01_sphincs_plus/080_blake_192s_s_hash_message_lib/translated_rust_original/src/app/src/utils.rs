extern "C" {
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn SPX_set_tree_height(addr: *mut uint32_t, tree_height: uint32_t);
    fn SPX_set_tree_index(addr: *mut uint32_t, tree_index: uint32_t);
    fn SPX_thash(
        out: *mut libc::c_uchar,
        in_0: *const libc::c_uchar,
        inblocks: libc::c_uint,
        ctx: *const spx_ctx,
        addr: *mut uint32_t,
    );
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct spx_ctx {
    pub pub_seed: [uint8_t; 16],
    pub sk_seed: [uint8_t; 16],
    pub tweaked512_rc64: [[uint64_t; 8]; 10],
    pub tweaked256_rc32: [[uint32_t; 8]; 10],
}
pub const SPX_N: libc::c_int = 16 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn SPX_ull_to_bytes(
    mut out: *mut libc::c_uchar,
    mut outlen: libc::c_uint,
    mut in_0: libc::c_ulonglong,
) {
    let mut i: libc::c_int = 0;
    i = outlen as libc::c_int - 1 as libc::c_int;
    while i >= 0 as libc::c_int {
        *out.offset(i as isize) = (in_0 & 0xff as libc::c_ulonglong) as libc::c_uchar;
        in_0 = in_0 >> 8 as libc::c_int;
        i -= 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn SPX_u32_to_bytes(mut out: *mut libc::c_uchar, mut in_0: uint32_t) {
    *out.offset(0 as libc::c_int as isize) =
        (in_0 >> 24 as libc::c_int) as libc::c_uchar;
    *out.offset(1 as libc::c_int as isize) =
        (in_0 >> 16 as libc::c_int) as libc::c_uchar;
    *out.offset(2 as libc::c_int as isize) =
        (in_0 >> 8 as libc::c_int) as libc::c_uchar;
    *out.offset(3 as libc::c_int as isize) = in_0 as libc::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_bytes_to_ull(
    mut in_0: *const libc::c_uchar,
    mut inlen: libc::c_uint,
) -> libc::c_ulonglong {
    let mut retval: libc::c_ulonglong = 0 as libc::c_ulonglong;
    let mut i: libc::c_uint = 0;
    i = 0 as libc::c_uint;
    while i < inlen {
        retval |= (*in_0.offset(i as isize) as libc::c_ulonglong)
            << (8 as libc::c_uint)
                .wrapping_mul(inlen.wrapping_sub(1 as libc::c_uint).wrapping_sub(i));
        i = i.wrapping_add(1);
    }
    return retval;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_compute_root(
    mut root: *mut libc::c_uchar,
    mut leaf: *const libc::c_uchar,
    mut leaf_idx: uint32_t,
    mut idx_offset: uint32_t,
    mut auth_path: *const libc::c_uchar,
    mut tree_height: uint32_t,
    mut ctx: *const spx_ctx,
    mut addr: *mut uint32_t,
) {
    let mut i: uint32_t = 0;
    let mut buffer: [libc::c_uchar; 32] = [0; 32];
    if leaf_idx & 1 as uint32_t != 0 {
        memcpy(
            (&raw mut buffer as *mut libc::c_uchar).offset(SPX_N as isize)
                as *mut libc::c_void,
            leaf as *const libc::c_void,
            SPX_N as size_t,
        );
        memcpy(
            &raw mut buffer as *mut libc::c_uchar as *mut libc::c_void,
            auth_path as *const libc::c_void,
            SPX_N as size_t,
        );
    } else {
        memcpy(
            &raw mut buffer as *mut libc::c_uchar as *mut libc::c_void,
            leaf as *const libc::c_void,
            SPX_N as size_t,
        );
        memcpy(
            (&raw mut buffer as *mut libc::c_uchar).offset(SPX_N as isize)
                as *mut libc::c_void,
            auth_path as *const libc::c_void,
            SPX_N as size_t,
        );
    }
    auth_path = auth_path.offset(SPX_N as isize);
    i = 0 as uint32_t;
    while i < tree_height.wrapping_sub(1 as uint32_t) {
        leaf_idx >>= 1 as libc::c_int;
        idx_offset >>= 1 as libc::c_int;
        SPX_set_tree_height(addr, i.wrapping_add(1 as uint32_t));
        SPX_set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));
        if leaf_idx & 1 as uint32_t != 0 {
            SPX_thash(
                (&raw mut buffer as *mut libc::c_uchar).offset(SPX_N as isize),
                &raw mut buffer as *mut libc::c_uchar,
                2 as libc::c_uint,
                ctx,
                addr,
            );
            memcpy(
                &raw mut buffer as *mut libc::c_uchar as *mut libc::c_void,
                auth_path as *const libc::c_void,
                SPX_N as size_t,
            );
        } else {
            SPX_thash(
                &raw mut buffer as *mut libc::c_uchar,
                &raw mut buffer as *mut libc::c_uchar,
                2 as libc::c_uint,
                ctx,
                addr,
            );
            memcpy(
                (&raw mut buffer as *mut libc::c_uchar).offset(SPX_N as isize)
                    as *mut libc::c_void,
                auth_path as *const libc::c_void,
                SPX_N as size_t,
            );
        }
        auth_path = auth_path.offset(SPX_N as isize);
        i = i.wrapping_add(1);
    }
    leaf_idx >>= 1 as libc::c_int;
    idx_offset >>= 1 as libc::c_int;
    SPX_set_tree_height(addr, tree_height);
    SPX_set_tree_index(addr, leaf_idx.wrapping_add(idx_offset));
    SPX_thash(
        root,
        &raw mut buffer as *mut libc::c_uchar,
        2 as libc::c_uint,
        ctx,
        addr,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_treehash(
    mut root: *mut libc::c_uchar,
    mut auth_path: *mut libc::c_uchar,
    mut ctx: *const spx_ctx,
    mut leaf_idx: uint32_t,
    mut idx_offset: uint32_t,
    mut tree_height: uint32_t,
    mut gen_leaf: Option<
        unsafe extern "C" fn(
            *mut libc::c_uchar,
            *const spx_ctx,
            uint32_t,
            *const uint32_t,
        ) -> (),
    >,
    mut tree_addr: *mut uint32_t,
) {
    let vla = tree_height
        .wrapping_add(1 as uint32_t)
        .wrapping_mul(16 as uint32_t) as usize;
    let mut stack: Vec<uint8_t> = ::std::vec::from_elem(0, vla);
    let vla_0 = tree_height.wrapping_add(1 as uint32_t) as usize;
    let mut heights: Vec<libc::c_uint> = ::std::vec::from_elem(0, vla_0);
    let mut offset: libc::c_uint = 0 as libc::c_uint;
    let mut idx: uint32_t = 0;
    let mut tree_idx: uint32_t = 0;
    idx = 0 as uint32_t;
    while idx < ((1 as libc::c_int) << tree_height) as uint32_t {
        gen_leaf.expect("non-null function pointer")(
            stack
                .as_mut_ptr()
                .offset(offset.wrapping_mul(SPX_N as libc::c_uint) as isize),
            ctx,
            idx.wrapping_add(idx_offset),
            tree_addr as *const uint32_t,
        );
        offset = offset.wrapping_add(1);
        *heights
            .as_mut_ptr()
            .offset(offset.wrapping_sub(1 as libc::c_uint) as isize) =
            0 as libc::c_uint;
        if leaf_idx ^ 0x1 as uint32_t == idx {
            memcpy(
                auth_path as *mut libc::c_void,
                stack.as_mut_ptr().offset(
                    offset
                        .wrapping_sub(1 as libc::c_uint)
                        .wrapping_mul(SPX_N as libc::c_uint) as isize,
                ) as *const libc::c_void,
                SPX_N as size_t,
            );
        }
        while offset >= 2 as libc::c_uint
            && *heights
                .as_mut_ptr()
                .offset(offset.wrapping_sub(1 as libc::c_uint) as isize)
                == *heights
                    .as_mut_ptr()
                    .offset(offset.wrapping_sub(2 as libc::c_uint) as isize)
        {
            tree_idx = idx
                >> (*heights
                    .as_mut_ptr()
                    .offset(offset.wrapping_sub(1 as libc::c_uint) as isize))
                .wrapping_add(1 as libc::c_uint);
            SPX_set_tree_height(
                tree_addr,
                (*heights
                    .as_mut_ptr()
                    .offset(offset.wrapping_sub(1 as libc::c_uint) as isize))
                .wrapping_add(1 as uint32_t),
            );
            SPX_set_tree_index(
                tree_addr,
                tree_idx.wrapping_add(
                    idx_offset
                        >> (*heights
                            .as_mut_ptr()
                            .offset(offset.wrapping_sub(1 as libc::c_uint) as isize))
                        .wrapping_add(1 as libc::c_uint),
                ),
            );
            SPX_thash(
                stack.as_mut_ptr().offset(
                    offset
                        .wrapping_sub(2 as libc::c_uint)
                        .wrapping_mul(SPX_N as libc::c_uint) as isize,
                ),
                stack.as_mut_ptr().offset(
                    offset
                        .wrapping_sub(2 as libc::c_uint)
                        .wrapping_mul(SPX_N as libc::c_uint) as isize,
                ),
                2 as libc::c_uint,
                ctx,
                tree_addr,
            );
            offset = offset.wrapping_sub(1);
            let ref mut fresh0 = *heights
                .as_mut_ptr()
                .offset(offset.wrapping_sub(1 as libc::c_uint) as isize);
            *fresh0 = (*fresh0).wrapping_add(1);
            if leaf_idx
                >> *heights
                    .as_mut_ptr()
                    .offset(offset.wrapping_sub(1 as libc::c_uint) as isize)
                ^ 0x1 as uint32_t
                == tree_idx
            {
                memcpy(
                    auth_path.offset(
                        (*heights
                            .as_mut_ptr()
                            .offset(offset.wrapping_sub(1 as libc::c_uint) as isize))
                        .wrapping_mul(SPX_N as libc::c_uint)
                            as isize,
                    ) as *mut libc::c_void,
                    stack.as_mut_ptr().offset(
                        offset
                            .wrapping_sub(1 as libc::c_uint)
                            .wrapping_mul(SPX_N as libc::c_uint)
                            as isize,
                    ) as *const libc::c_void,
                    SPX_N as size_t,
                );
            }
        }
        idx = idx.wrapping_add(1);
    }
    memcpy(
        root as *mut libc::c_void,
        stack.as_mut_ptr() as *const libc::c_void,
        SPX_N as size_t,
    );
}
