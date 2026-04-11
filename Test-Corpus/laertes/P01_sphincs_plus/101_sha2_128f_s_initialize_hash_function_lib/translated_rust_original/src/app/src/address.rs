extern "C" {
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn SPX_ull_to_bytes(
        out: *mut libc::c_uchar,
        outlen: libc::c_uint,
        in_0: libc::c_ulonglong,
    );
    fn SPX_u32_to_bytes(out: *mut libc::c_uchar, in_0: uint32_t);
}
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type size_t = usize;
pub const SPX_OFFSET_LAYER: libc::c_int = 3 as libc::c_int;
pub const SPX_OFFSET_TREE: libc::c_int = 8 as libc::c_int;
pub const SPX_OFFSET_TYPE: libc::c_int = 19 as libc::c_int;
pub const SPX_OFFSET_KP_ADDR: libc::c_int = 20 as libc::c_int;
pub const SPX_OFFSET_CHAIN_ADDR: libc::c_int = 27 as libc::c_int;
pub const SPX_OFFSET_HASH_ADDR: libc::c_int = 31 as libc::c_int;
pub const SPX_OFFSET_TREE_HGT: libc::c_int = 27 as libc::c_int;
pub const SPX_OFFSET_TREE_INDEX: libc::c_int = 28 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn SPX_set_layer_addr(mut addr: *mut uint32_t, mut layer: uint32_t) {
    *(addr as *mut libc::c_uchar).offset(SPX_OFFSET_LAYER as isize) =
        layer as libc::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_addr(mut addr: *mut uint32_t, mut tree: uint64_t) {
    SPX_ull_to_bytes(
        (addr as *mut libc::c_uchar).offset(SPX_OFFSET_TREE as isize)
            as *mut libc::c_uchar,
        8 as libc::c_uint,
        tree as libc::c_ulonglong,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_type(mut addr: *mut uint32_t, mut type_0: uint32_t) {
    *(addr as *mut libc::c_uchar).offset(SPX_OFFSET_TYPE as isize) =
        type_0 as libc::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_copy_subtree_addr(mut out: *mut uint32_t, mut in_0: *const uint32_t) {
    memcpy(
        out as *mut libc::c_void,
        in_0 as *const libc::c_void,
        (SPX_OFFSET_TREE + 8 as libc::c_int) as size_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_keypair_addr(mut addr: *mut uint32_t, mut keypair: uint32_t) {
    SPX_u32_to_bytes(
        (addr as *mut libc::c_uchar).offset(SPX_OFFSET_KP_ADDR as isize)
            as *mut libc::c_uchar,
        keypair,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_copy_keypair_addr(mut out: *mut uint32_t, mut in_0: *const uint32_t) {
    memcpy(
        out as *mut libc::c_void,
        in_0 as *const libc::c_void,
        (SPX_OFFSET_TREE + 8 as libc::c_int) as size_t,
    );
    memcpy(
        (out as *mut libc::c_uchar).offset(SPX_OFFSET_KP_ADDR as isize)
            as *mut libc::c_void,
        (in_0 as *mut libc::c_uchar).offset(SPX_OFFSET_KP_ADDR as isize)
            as *const libc::c_void,
        4 as size_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_chain_addr(mut addr: *mut uint32_t, mut chain: uint32_t) {
    *(addr as *mut libc::c_uchar).offset(SPX_OFFSET_CHAIN_ADDR as isize) =
        chain as libc::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_hash_addr(mut addr: *mut uint32_t, mut hash: uint32_t) {
    *(addr as *mut libc::c_uchar).offset(SPX_OFFSET_HASH_ADDR as isize) =
        hash as libc::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_height(mut addr: *mut uint32_t, mut tree_height: uint32_t) {
    *(addr as *mut libc::c_uchar).offset(SPX_OFFSET_TREE_HGT as isize) =
        tree_height as libc::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_index(mut addr: *mut uint32_t, mut tree_index: uint32_t) {
    SPX_u32_to_bytes(
        (addr as *mut libc::c_uchar).offset(SPX_OFFSET_TREE_INDEX as isize)
            as *mut libc::c_uchar,
        tree_index,
    );
}
