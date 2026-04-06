extern "C" {
    fn SPX_set_type(addr: *mut uint32_t, type_0: uint32_t);
    fn SPX_copy_keypair_addr(out: *mut uint32_t, in_0: *const uint32_t);
    fn SPX_set_tree_height(addr: *mut uint32_t, tree_height: uint32_t);
    fn SPX_set_tree_index(addr: *mut uint32_t, tree_index: uint32_t);
    fn SPX_prf_addr(out: *mut ::core::ffi::c_uchar, ctx: *const spx_ctx, addr: *const uint32_t);
    fn SPX_thash(
        out: *mut ::core::ffi::c_uchar,
        in_0: *const ::core::ffi::c_uchar,
        inblocks: ::core::ffi::c_uint,
        ctx: *const spx_ctx,
        addr: *mut uint32_t,
    );
    fn SPX_compute_root(
        root: *mut ::core::ffi::c_uchar,
        leaf: *const ::core::ffi::c_uchar,
        leaf_idx: uint32_t,
        idx_offset: uint32_t,
        auth_path: *const ::core::ffi::c_uchar,
        tree_height: uint32_t,
        ctx: *const spx_ctx,
        addr: *mut uint32_t,
    );
    fn SPX_fors_treehashx1(
        root: *mut ::core::ffi::c_uchar,
        auth_path: *mut ::core::ffi::c_uchar,
        ctx: *const spx_ctx,
        leaf_idx: uint32_t,
        idx_offset: uint32_t,
        tree_height: uint32_t,
        tree_addrx4: *mut uint32_t,
        info: *mut leaf_info_x1,
    );
}
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
pub const SPX_FORS_HEIGHT: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const SPX_FORS_TREES: ::core::ffi::c_int = 14 as ::core::ffi::c_int;
pub const SPX_ADDR_TYPE_FORSTREE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const SPX_ADDR_TYPE_FORSPK: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const SPX_ADDR_TYPE_FORSPRF: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
unsafe extern "C" fn fors_gen_sk(
    mut sk: *mut ::core::ffi::c_uchar,
    mut ctx: *const spx_ctx,
    mut fors_leaf_addr: *mut uint32_t,
) {
    SPX_prf_addr(sk, ctx, fors_leaf_addr as *const uint32_t);
}
unsafe extern "C" fn fors_sk_to_leaf(
    mut leaf: *mut ::core::ffi::c_uchar,
    mut sk: *const ::core::ffi::c_uchar,
    mut ctx: *const spx_ctx,
    mut fors_leaf_addr: *mut uint32_t,
) {
    SPX_thash(leaf, sk, 1 as ::core::ffi::c_uint, ctx, fors_leaf_addr);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_fors_gen_leafx1(
    mut leaf: *mut ::core::ffi::c_uchar,
    mut ctx: *const spx_ctx,
    mut addr_idx: uint32_t,
    mut info: *mut fors_gen_leaf_info,
) {
    let mut fors_info: *mut fors_gen_leaf_info = info as *mut fors_gen_leaf_info;
    let mut fors_leaf_addr: *mut uint32_t = &raw mut (*fors_info).leaf_addrx as *mut uint32_t;
    SPX_set_tree_index(fors_leaf_addr as *mut uint32_t, addr_idx);
    SPX_set_type(
        fors_leaf_addr as *mut uint32_t,
        SPX_ADDR_TYPE_FORSPRF as uint32_t,
    );
    fors_gen_sk(leaf, ctx, fors_leaf_addr as *mut uint32_t);
    SPX_set_type(
        fors_leaf_addr as *mut uint32_t,
        SPX_ADDR_TYPE_FORSTREE as uint32_t,
    );
    fors_sk_to_leaf(leaf, leaf, ctx, fors_leaf_addr as *mut uint32_t);
}
unsafe extern "C" fn message_to_indices(
    mut indices: *mut uint32_t,
    mut m: *const ::core::ffi::c_uchar,
) {
    let mut i: ::core::ffi::c_uint = 0;
    let mut j: ::core::ffi::c_uint = 0;
    let mut offset: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    i = 0 as ::core::ffi::c_uint;
    while i < SPX_FORS_TREES as ::core::ffi::c_uint {
        *indices.offset(i as isize) = 0 as uint32_t;
        j = 0 as ::core::ffi::c_uint;
        while j < SPX_FORS_HEIGHT as ::core::ffi::c_uint {
            let ref mut fresh0 = *indices.offset(i as isize);
            *fresh0 = (*fresh0 as ::core::ffi::c_uint
                ^ ((*m.offset((offset >> 3 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                    >> (offset & 0x7 as ::core::ffi::c_uint))
                    as ::core::ffi::c_uint
                    & 1 as ::core::ffi::c_uint)
                    << j) as uint32_t;
            offset = offset.wrapping_add(1);
            j = j.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
}
#[no_mangle]
pub unsafe extern "C" fn SPX_fors_sign(
    mut sig: *mut ::core::ffi::c_uchar,
    mut pk: *mut ::core::ffi::c_uchar,
    mut m: *const ::core::ffi::c_uchar,
    mut ctx: *const spx_ctx,
    mut fors_addr: *const uint32_t,
) {
    let mut indices: [uint32_t; 14] = [0; 14];
    let mut roots: [::core::ffi::c_uchar; 224] = [0; 224];
    let mut fors_tree_addr: [uint32_t; 8] =
        [0 as ::core::ffi::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0];
    let mut fors_info: fors_gen_leaf_info = fors_gen_leaf_info {
        leaf_addrx: [0 as ::core::ffi::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0],
    };
    let mut fors_leaf_addr: *mut uint32_t = &raw mut fors_info.leaf_addrx as *mut uint32_t;
    let mut fors_pk_addr: [uint32_t; 8] =
        [0 as ::core::ffi::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0];
    let mut idx_offset: uint32_t = 0;
    let mut i: ::core::ffi::c_uint = 0;
    SPX_copy_keypair_addr(&raw mut fors_tree_addr as *mut uint32_t, fors_addr);
    SPX_copy_keypair_addr(fors_leaf_addr as *mut uint32_t, fors_addr);
    SPX_copy_keypair_addr(&raw mut fors_pk_addr as *mut uint32_t, fors_addr);
    SPX_set_type(
        &raw mut fors_pk_addr as *mut uint32_t,
        SPX_ADDR_TYPE_FORSPK as uint32_t,
    );
    message_to_indices(&raw mut indices as *mut uint32_t, m);
    i = 0 as ::core::ffi::c_uint;
    while i < SPX_FORS_TREES as ::core::ffi::c_uint {
        idx_offset = i
            .wrapping_mul(((1 as ::core::ffi::c_int) << SPX_FORS_HEIGHT) as ::core::ffi::c_uint)
            as uint32_t;
        SPX_set_tree_height(&raw mut fors_tree_addr as *mut uint32_t, 0 as uint32_t);
        SPX_set_tree_index(
            &raw mut fors_tree_addr as *mut uint32_t,
            indices[i as usize].wrapping_add(idx_offset),
        );
        SPX_set_type(
            &raw mut fors_tree_addr as *mut uint32_t,
            SPX_ADDR_TYPE_FORSPRF as uint32_t,
        );
        fors_gen_sk(sig, ctx, &raw mut fors_tree_addr as *mut uint32_t);
        SPX_set_type(
            &raw mut fors_tree_addr as *mut uint32_t,
            SPX_ADDR_TYPE_FORSTREE as uint32_t,
        );
        sig = sig.offset(SPX_N as isize);
        SPX_fors_treehashx1(
            (&raw mut roots as *mut ::core::ffi::c_uchar)
                .offset(i.wrapping_mul(SPX_N as ::core::ffi::c_uint) as isize),
            sig,
            ctx,
            indices[i as usize],
            idx_offset,
            SPX_FORS_HEIGHT as uint32_t,
            &raw mut fors_tree_addr as *mut uint32_t,
            &raw mut fors_info as *mut leaf_info_x1,
        );
        sig = sig.offset((SPX_N * SPX_FORS_HEIGHT) as isize);
        i = i.wrapping_add(1);
    }
    SPX_thash(
        pk,
        &raw mut roots as *mut ::core::ffi::c_uchar,
        SPX_FORS_TREES as ::core::ffi::c_uint,
        ctx,
        &raw mut fors_pk_addr as *mut uint32_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_fors_pk_from_sig(
    mut pk: *mut ::core::ffi::c_uchar,
    mut sig: *const ::core::ffi::c_uchar,
    mut m: *const ::core::ffi::c_uchar,
    mut ctx: *const spx_ctx,
    mut fors_addr: *const uint32_t,
) {
    let mut indices: [uint32_t; 14] = [0; 14];
    let mut roots: [::core::ffi::c_uchar; 224] = [0; 224];
    let mut leaf: [::core::ffi::c_uchar; 16] = [0; 16];
    let mut fors_tree_addr: [uint32_t; 8] =
        [0 as ::core::ffi::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0];
    let mut fors_pk_addr: [uint32_t; 8] =
        [0 as ::core::ffi::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0];
    let mut idx_offset: uint32_t = 0;
    let mut i: ::core::ffi::c_uint = 0;
    SPX_copy_keypair_addr(&raw mut fors_tree_addr as *mut uint32_t, fors_addr);
    SPX_copy_keypair_addr(&raw mut fors_pk_addr as *mut uint32_t, fors_addr);
    SPX_set_type(
        &raw mut fors_tree_addr as *mut uint32_t,
        SPX_ADDR_TYPE_FORSTREE as uint32_t,
    );
    SPX_set_type(
        &raw mut fors_pk_addr as *mut uint32_t,
        SPX_ADDR_TYPE_FORSPK as uint32_t,
    );
    message_to_indices(&raw mut indices as *mut uint32_t, m);
    i = 0 as ::core::ffi::c_uint;
    while i < SPX_FORS_TREES as ::core::ffi::c_uint {
        idx_offset = i
            .wrapping_mul(((1 as ::core::ffi::c_int) << SPX_FORS_HEIGHT) as ::core::ffi::c_uint)
            as uint32_t;
        SPX_set_tree_height(&raw mut fors_tree_addr as *mut uint32_t, 0 as uint32_t);
        SPX_set_tree_index(
            &raw mut fors_tree_addr as *mut uint32_t,
            indices[i as usize].wrapping_add(idx_offset),
        );
        fors_sk_to_leaf(
            &raw mut leaf as *mut ::core::ffi::c_uchar,
            sig,
            ctx,
            &raw mut fors_tree_addr as *mut uint32_t,
        );
        sig = sig.offset(SPX_N as isize);
        SPX_compute_root(
            (&raw mut roots as *mut ::core::ffi::c_uchar)
                .offset(i.wrapping_mul(SPX_N as ::core::ffi::c_uint) as isize),
            &raw mut leaf as *mut ::core::ffi::c_uchar,
            indices[i as usize],
            idx_offset,
            sig,
            SPX_FORS_HEIGHT as uint32_t,
            ctx,
            &raw mut fors_tree_addr as *mut uint32_t,
        );
        sig = sig.offset((SPX_N * SPX_FORS_HEIGHT) as isize);
        i = i.wrapping_add(1);
    }
    SPX_thash(
        pk,
        &raw mut roots as *mut ::core::ffi::c_uchar,
        SPX_FORS_TREES as ::core::ffi::c_uint,
        ctx,
        &raw mut fors_pk_addr as *mut uint32_t,
    );
}
