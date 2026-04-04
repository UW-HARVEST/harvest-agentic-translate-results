extern "C" {
    fn SPX_set_layer_addr(addr: *mut uint32_t, layer: uint32_t);
    fn SPX_set_type(addr: *mut uint32_t, type_0: uint32_t);
    fn SPX_copy_subtree_addr(out: *mut uint32_t, in_0: *const uint32_t);
    fn SPX_wots_treehashx1(
        root: *mut ::core::ffi::c_uchar,
        auth_path: *mut ::core::ffi::c_uchar,
        ctx: *const spx_ctx,
        leaf_idx: uint32_t,
        idx_offset: uint32_t,
        tree_height: uint32_t,
        tree_addrx4: *mut uint32_t,
        info: *mut leaf_info_x1,
    );
    fn SPX_chain_lengths(lengths: *mut ::core::ffi::c_uint, msg: *const ::core::ffi::c_uchar);
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
pub struct leaf_info_x1 {
    pub wots_sig: *mut ::core::ffi::c_uchar,
    pub wots_sign_leaf: uint32_t,
    pub wots_steps: *mut uint32_t,
    pub leaf_addr: [uint32_t; 8],
    pub pk_addr: [uint32_t; 8],
}
pub const SPX_N: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const SPX_FULL_HEIGHT: ::core::ffi::c_int = 63 as ::core::ffi::c_int;
pub const SPX_D: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const SPX_WOTS_LOGW: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const SPX_WOTS_LEN1: ::core::ffi::c_int = 8 as ::core::ffi::c_int * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const SPX_WOTS_LEN: ::core::ffi::c_int = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: ::core::ffi::c_int = SPX_WOTS_LEN * SPX_N;
pub const SPX_TREE_HEIGHT: ::core::ffi::c_int = SPX_FULL_HEIGHT / SPX_D;
pub const SPX_ADDR_TYPE_WOTSPK: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const SPX_ADDR_TYPE_HASHTREE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn SPX_merkle_sign(
    mut sig: *mut uint8_t,
    mut root: *mut ::core::ffi::c_uchar,
    mut ctx: *const spx_ctx,
    mut wots_addr: *mut uint32_t,
    mut tree_addr: *mut uint32_t,
    mut idx_leaf: uint32_t,
) {
    let mut auth_path: *mut ::core::ffi::c_uchar = sig.offset(SPX_WOTS_BYTES as isize);
    let mut info: leaf_info_x1 = leaf_info_x1 {
        wots_sig: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        wots_sign_leaf: 0,
        wots_steps: ::core::ptr::null_mut::<uint32_t>(),
        leaf_addr: [0; 8],
        pk_addr: [0; 8],
    };
    let mut steps: [::core::ffi::c_uint; 35] = [0; 35];
    info.wots_sig = sig as *mut ::core::ffi::c_uchar;
    SPX_chain_lengths(&raw mut steps as *mut ::core::ffi::c_uint, root);
    info.wots_steps = &raw mut steps as *mut ::core::ffi::c_uint as *mut uint32_t;
    SPX_set_type(
        tree_addr.offset(0 as ::core::ffi::c_int as isize) as *mut uint32_t,
        SPX_ADDR_TYPE_HASHTREE as uint32_t,
    );
    SPX_set_type(
        (&raw mut info.pk_addr as *mut uint32_t).offset(0 as ::core::ffi::c_int as isize)
            as *mut uint32_t,
        SPX_ADDR_TYPE_WOTSPK as uint32_t,
    );
    SPX_copy_subtree_addr(
        (&raw mut info.leaf_addr as *mut uint32_t).offset(0 as ::core::ffi::c_int as isize)
            as *mut uint32_t,
        wots_addr as *const uint32_t,
    );
    SPX_copy_subtree_addr(
        (&raw mut info.pk_addr as *mut uint32_t).offset(0 as ::core::ffi::c_int as isize)
            as *mut uint32_t,
        wots_addr as *const uint32_t,
    );
    info.wots_sign_leaf = idx_leaf;
    SPX_wots_treehashx1(
        root,
        auth_path,
        ctx,
        idx_leaf,
        0 as uint32_t,
        SPX_TREE_HEIGHT as uint32_t,
        tree_addr,
        &raw mut info,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_merkle_gen_root(
    mut root: *mut ::core::ffi::c_uchar,
    mut ctx: *const spx_ctx,
) {
    let mut auth_path: [::core::ffi::c_uchar; 704] = [0; 704];
    let mut top_tree_addr: [uint32_t; 8] =
        [0 as ::core::ffi::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0];
    let mut wots_addr: [uint32_t; 8] = [0 as ::core::ffi::c_int as uint32_t, 0, 0, 0, 0, 0, 0, 0];
    SPX_set_layer_addr(
        &raw mut top_tree_addr as *mut uint32_t,
        (SPX_D - 1 as ::core::ffi::c_int) as uint32_t,
    );
    SPX_set_layer_addr(
        &raw mut wots_addr as *mut uint32_t,
        (SPX_D - 1 as ::core::ffi::c_int) as uint32_t,
    );
    SPX_merkle_sign(
        &raw mut auth_path as *mut uint8_t,
        root,
        ctx,
        &raw mut wots_addr as *mut uint32_t,
        &raw mut top_tree_addr as *mut uint32_t,
        !(0 as ::core::ffi::c_int) as uint32_t,
    );
}
