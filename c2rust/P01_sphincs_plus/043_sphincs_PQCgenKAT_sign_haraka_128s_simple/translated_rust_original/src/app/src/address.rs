extern "C" {
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn SPX_ull_to_bytes(
        out: *mut ::core::ffi::c_uchar,
        outlen: ::core::ffi::c_uint,
        in_0: ::core::ffi::c_ulonglong,
    );
    fn SPX_u32_to_bytes(out: *mut ::core::ffi::c_uchar, in_0: uint32_t);
}
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type size_t = usize;
pub const SPX_OFFSET_LAYER: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const SPX_OFFSET_TREE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const SPX_OFFSET_TYPE: ::core::ffi::c_int = 19 as ::core::ffi::c_int;
pub const SPX_OFFSET_KP_ADDR: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const SPX_OFFSET_CHAIN_ADDR: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
pub const SPX_OFFSET_HASH_ADDR: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
pub const SPX_OFFSET_TREE_HGT: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
pub const SPX_OFFSET_TREE_INDEX: ::core::ffi::c_int = 28 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn SPX_set_layer_addr(mut addr: *mut uint32_t, mut layer: uint32_t) {
    *(addr as *mut ::core::ffi::c_uchar).offset(SPX_OFFSET_LAYER as isize) =
        layer as ::core::ffi::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_addr(mut addr: *mut uint32_t, mut tree: uint64_t) {
    SPX_ull_to_bytes(
        (addr as *mut ::core::ffi::c_uchar).offset(SPX_OFFSET_TREE as isize)
            as *mut ::core::ffi::c_uchar,
        8 as ::core::ffi::c_uint,
        tree as ::core::ffi::c_ulonglong,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_type(mut addr: *mut uint32_t, mut type_0: uint32_t) {
    *(addr as *mut ::core::ffi::c_uchar).offset(SPX_OFFSET_TYPE as isize) =
        type_0 as ::core::ffi::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_copy_subtree_addr(mut out: *mut uint32_t, mut in_0: *const uint32_t) {
    memcpy(
        out as *mut ::core::ffi::c_void,
        in_0 as *const ::core::ffi::c_void,
        (SPX_OFFSET_TREE + 8 as ::core::ffi::c_int) as size_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_keypair_addr(mut addr: *mut uint32_t, mut keypair: uint32_t) {
    SPX_u32_to_bytes(
        (addr as *mut ::core::ffi::c_uchar).offset(SPX_OFFSET_KP_ADDR as isize)
            as *mut ::core::ffi::c_uchar,
        keypair,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_copy_keypair_addr(mut out: *mut uint32_t, mut in_0: *const uint32_t) {
    memcpy(
        out as *mut ::core::ffi::c_void,
        in_0 as *const ::core::ffi::c_void,
        (SPX_OFFSET_TREE + 8 as ::core::ffi::c_int) as size_t,
    );
    memcpy(
        (out as *mut ::core::ffi::c_uchar).offset(SPX_OFFSET_KP_ADDR as isize)
            as *mut ::core::ffi::c_void,
        (in_0 as *mut ::core::ffi::c_uchar).offset(SPX_OFFSET_KP_ADDR as isize)
            as *const ::core::ffi::c_void,
        4 as size_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_chain_addr(mut addr: *mut uint32_t, mut chain: uint32_t) {
    *(addr as *mut ::core::ffi::c_uchar).offset(SPX_OFFSET_CHAIN_ADDR as isize) =
        chain as ::core::ffi::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_hash_addr(mut addr: *mut uint32_t, mut hash: uint32_t) {
    *(addr as *mut ::core::ffi::c_uchar).offset(SPX_OFFSET_HASH_ADDR as isize) =
        hash as ::core::ffi::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_height(mut addr: *mut uint32_t, mut tree_height: uint32_t) {
    *(addr as *mut ::core::ffi::c_uchar).offset(SPX_OFFSET_TREE_HGT as isize) =
        tree_height as ::core::ffi::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn SPX_set_tree_index(mut addr: *mut uint32_t, mut tree_index: uint32_t) {
    SPX_u32_to_bytes(
        (addr as *mut ::core::ffi::c_uchar).offset(SPX_OFFSET_TREE_INDEX as isize)
            as *mut ::core::ffi::c_uchar,
        tree_index,
    );
}
