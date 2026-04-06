extern "C" {
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn SPX_set_tree_height(addr: *mut uint32_t, tree_height: uint32_t);
    fn SPX_set_tree_index(addr: *mut uint32_t, tree_index: uint32_t);
    fn SPX_fors_gen_leafx1(
        leaf: *mut ::core::ffi::c_uchar,
        ctx: *const spx_ctx,
        addr_idx: uint32_t,
        info: *mut fors_gen_leaf_info,
    );
    fn SPX_thash(
        out: *mut ::core::ffi::c_uchar,
        in_0: *const ::core::ffi::c_uchar,
        inblocks: ::core::ffi::c_uint,
        ctx: *const spx_ctx,
        addr: *mut uint32_t,
    );
    fn SPX_wots_gen_leafx1(
        dest: *mut ::core::ffi::c_uchar,
        ctx: *const spx_ctx,
        leaf_idx: uint32_t,
        v_info: *mut leaf_info_x1,
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct fors_gen_leaf_info {
    pub leaf_addrx: [uint32_t; 8],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct leaf_info_x1 {
    pub wots_sig: *mut ::core::ffi::c_uchar,
    pub wots_sign_leaf: uint32_t,
    pub wots_steps: *mut uint32_t,
    pub leaf_addr: [uint32_t; 8],
    pub pk_addr: [uint32_t; 8],
}
pub const SPX_N: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn SPX_wots_treehashx1(
    mut root: *mut ::core::ffi::c_uchar,
    mut auth_path: *mut ::core::ffi::c_uchar,
    mut ctx: *const spx_ctx,
    mut leaf_idx: uint32_t,
    mut idx_offset: uint32_t,
    mut tree_height: uint32_t,
    mut tree_addr: *mut uint32_t,
    mut info: *mut leaf_info_x1,
) {
    let vla = tree_height.wrapping_mul(16 as uint32_t) as usize;
    let mut stack: Vec<uint8_t> = ::std::vec::from_elem(0, vla);
    let mut idx: uint32_t = 0;
    let mut max_idx: uint32_t =
        (((1 as ::core::ffi::c_int) << tree_height) - 1 as ::core::ffi::c_int) as uint32_t;
    idx = 0 as uint32_t;
    loop {
        let mut current: [::core::ffi::c_uchar; 32] = [0; 32];
        SPX_wots_gen_leafx1(
            (&raw mut current as *mut ::core::ffi::c_uchar).offset(SPX_N as isize)
                as *mut ::core::ffi::c_uchar,
            ctx,
            idx.wrapping_add(idx_offset),
            info,
        );
        let mut internal_idx_offset: uint32_t = idx_offset;
        let mut internal_idx: uint32_t = idx;
        let mut internal_leaf: uint32_t = leaf_idx;
        let mut h: uint32_t = 0;
        h = 0 as uint32_t;
        loop {
            if h == tree_height {
                memcpy(
                    root as *mut ::core::ffi::c_void,
                    (&raw mut current as *mut ::core::ffi::c_uchar).offset(SPX_N as isize)
                        as *mut ::core::ffi::c_uchar
                        as *const ::core::ffi::c_void,
                    SPX_N as size_t,
                );
                return;
            }
            if internal_idx ^ internal_leaf == 0x1 as uint32_t {
                memcpy(
                    auth_path.offset(h.wrapping_mul(SPX_N as uint32_t) as isize)
                        as *mut ::core::ffi::c_uchar
                        as *mut ::core::ffi::c_void,
                    (&raw mut current as *mut ::core::ffi::c_uchar).offset(SPX_N as isize)
                        as *mut ::core::ffi::c_uchar
                        as *const ::core::ffi::c_void,
                    SPX_N as size_t,
                );
            }
            if internal_idx & 1 as uint32_t == 0 as uint32_t && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1 as ::core::ffi::c_int;
            SPX_set_tree_height(tree_addr, h.wrapping_add(1 as uint32_t));
            SPX_set_tree_index(
                tree_addr,
                internal_idx
                    .wrapping_div(2 as uint32_t)
                    .wrapping_add(internal_idx_offset),
            );
            let mut left: *mut ::core::ffi::c_uchar = stack
                .as_mut_ptr()
                .offset(h.wrapping_mul(SPX_N as uint32_t) as isize)
                as *mut ::core::ffi::c_uchar;
            memcpy(
                (&raw mut current as *mut ::core::ffi::c_uchar)
                    .offset(0 as ::core::ffi::c_int as isize)
                    as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
                left as *const ::core::ffi::c_void,
                SPX_N as size_t,
            );
            SPX_thash(
                (&raw mut current as *mut ::core::ffi::c_uchar)
                    .offset((1 as ::core::ffi::c_int * SPX_N) as isize)
                    as *mut ::core::ffi::c_uchar,
                (&raw mut current as *mut ::core::ffi::c_uchar)
                    .offset((0 as ::core::ffi::c_int * SPX_N) as isize)
                    as *mut ::core::ffi::c_uchar,
                2 as ::core::ffi::c_uint,
                ctx,
                tree_addr,
            );
            h = h.wrapping_add(1);
            internal_idx >>= 1 as ::core::ffi::c_int;
            internal_leaf >>= 1 as ::core::ffi::c_int;
        }
        memcpy(
            stack
                .as_mut_ptr()
                .offset(h.wrapping_mul(SPX_N as uint32_t) as isize) as *mut uint8_t
                as *mut ::core::ffi::c_void,
            (&raw mut current as *mut ::core::ffi::c_uchar).offset(SPX_N as isize)
                as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
            SPX_N as size_t,
        );
        idx = idx.wrapping_add(1);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SPX_fors_treehashx1(
    mut root: *mut ::core::ffi::c_uchar,
    mut auth_path: *mut ::core::ffi::c_uchar,
    mut ctx: *const spx_ctx,
    mut leaf_idx: uint32_t,
    mut idx_offset: uint32_t,
    mut tree_height: uint32_t,
    mut tree_addr: *mut uint32_t,
    mut info: *mut leaf_info_x1,
) {
    let vla = tree_height.wrapping_mul(16 as uint32_t) as usize;
    let mut stack: Vec<uint8_t> = ::std::vec::from_elem(0, vla);
    let mut idx: uint32_t = 0;
    let mut max_idx: uint32_t =
        (((1 as ::core::ffi::c_int) << tree_height) - 1 as ::core::ffi::c_int) as uint32_t;
    idx = 0 as uint32_t;
    loop {
        let mut current: [::core::ffi::c_uchar; 32] = [0; 32];
        SPX_fors_gen_leafx1(
            (&raw mut current as *mut ::core::ffi::c_uchar).offset(SPX_N as isize)
                as *mut ::core::ffi::c_uchar,
            ctx,
            idx.wrapping_add(idx_offset),
            info as *mut fors_gen_leaf_info,
        );
        let mut internal_idx_offset: uint32_t = idx_offset;
        let mut internal_idx: uint32_t = idx;
        let mut internal_leaf: uint32_t = leaf_idx;
        let mut h: uint32_t = 0;
        h = 0 as uint32_t;
        loop {
            if h == tree_height {
                memcpy(
                    root as *mut ::core::ffi::c_void,
                    (&raw mut current as *mut ::core::ffi::c_uchar).offset(SPX_N as isize)
                        as *mut ::core::ffi::c_uchar
                        as *const ::core::ffi::c_void,
                    SPX_N as size_t,
                );
                return;
            }
            if internal_idx ^ internal_leaf == 0x1 as uint32_t {
                memcpy(
                    auth_path.offset(h.wrapping_mul(SPX_N as uint32_t) as isize)
                        as *mut ::core::ffi::c_uchar
                        as *mut ::core::ffi::c_void,
                    (&raw mut current as *mut ::core::ffi::c_uchar).offset(SPX_N as isize)
                        as *mut ::core::ffi::c_uchar
                        as *const ::core::ffi::c_void,
                    SPX_N as size_t,
                );
            }
            if internal_idx & 1 as uint32_t == 0 as uint32_t && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1 as ::core::ffi::c_int;
            SPX_set_tree_height(tree_addr, h.wrapping_add(1 as uint32_t));
            SPX_set_tree_index(
                tree_addr,
                internal_idx
                    .wrapping_div(2 as uint32_t)
                    .wrapping_add(internal_idx_offset),
            );
            let mut left: *mut ::core::ffi::c_uchar = stack
                .as_mut_ptr()
                .offset(h.wrapping_mul(SPX_N as uint32_t) as isize)
                as *mut ::core::ffi::c_uchar;
            memcpy(
                (&raw mut current as *mut ::core::ffi::c_uchar)
                    .offset(0 as ::core::ffi::c_int as isize)
                    as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
                left as *const ::core::ffi::c_void,
                SPX_N as size_t,
            );
            SPX_thash(
                (&raw mut current as *mut ::core::ffi::c_uchar)
                    .offset((1 as ::core::ffi::c_int * SPX_N) as isize)
                    as *mut ::core::ffi::c_uchar,
                (&raw mut current as *mut ::core::ffi::c_uchar)
                    .offset((0 as ::core::ffi::c_int * SPX_N) as isize)
                    as *mut ::core::ffi::c_uchar,
                2 as ::core::ffi::c_uint,
                ctx,
                tree_addr,
            );
            h = h.wrapping_add(1);
            internal_idx >>= 1 as ::core::ffi::c_int;
            internal_leaf >>= 1 as ::core::ffi::c_int;
        }
        memcpy(
            stack
                .as_mut_ptr()
                .offset(h.wrapping_mul(SPX_N as uint32_t) as isize) as *mut uint8_t
                as *mut ::core::ffi::c_void,
            (&raw mut current as *mut ::core::ffi::c_uchar).offset(SPX_N as isize)
                as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
            SPX_N as size_t,
        );
        idx = idx.wrapping_add(1);
    }
}
