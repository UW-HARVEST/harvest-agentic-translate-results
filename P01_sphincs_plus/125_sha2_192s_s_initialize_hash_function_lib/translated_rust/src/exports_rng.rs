use crate::context::AesXofStruct;

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) {
    let buf = unsafe { core::slice::from_raw_parts_mut(x, xlen as usize) };
    crate::rng::randombytes_urandom(buf, xlen);
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *const u8,
    key: *mut u8,
    v: *mut u8,
) {
    let k = unsafe { &mut *(key as *mut [u8; 32]) };
    let vv = unsafe { &mut *(v as *mut [u8; 16]) };
    crate::rng::aes256_ctr_drbg_update_internal(provided_data, k, vv);
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut AesXofStruct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: u64,
) -> i32 {
    let c = unsafe { &mut *ctx };
    let s = unsafe { core::slice::from_raw_parts(seed, 32) };
    let d = unsafe { core::slice::from_raw_parts(diversifier, 8) };
    crate::rng::seedexpander_init_internal(c, s, d, maxlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(
    ctx: *mut AesXofStruct,
    x: *mut u8,
    xlen: u64,
) -> i32 {
    let c = unsafe { &mut *ctx };
    let buf = unsafe { core::slice::from_raw_parts_mut(x, xlen as usize) };
    crate::rng::seedexpander_internal(c, buf, xlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let ei = unsafe { core::slice::from_raw_parts(entropy_input, 48) };
    crate::rng::randombytes_init_internal(ei, personalization_string);
}

// wots_gen_leafx1 export
#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_gen_leafx1(
    dest: *mut u8,
    ctx: *const crate::context::SpxCtx,
    leaf_idx: u32,
    v_info: *mut crate::context::LeafInfoX1,
) {
    let d = unsafe { core::slice::from_raw_parts_mut(dest, crate::params::SPX_N) };
    let c = unsafe { &*ctx };
    let i = unsafe { &mut *v_info };
    crate::wotsx1::wots_gen_leafx1_internal(d, c, leaf_idx, i);
}

// fors_gen_leafx1 export
#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8,
    ctx: *const crate::context::SpxCtx,
    addr_idx: u32,
    info: *mut crate::context::ForsGenLeafInfo,
) {
    let l = unsafe { core::slice::from_raw_parts_mut(leaf, crate::params::SPX_N) };
    let c = unsafe { &*ctx };
    let i = unsafe { &mut *info };
    crate::utilsx1::fors_gen_leafx1_internal(l, c, addr_idx, i);
}

// treehash exports
#[unsafe(no_mangle)]
pub extern "C" fn SPX_wots_treehashx1(
    root: *mut u8, auth_path: *mut u8,
    ctx: *const crate::context::SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: *mut u32,
    info: *mut crate::context::LeafInfoX1,
) {
    let r = unsafe { core::slice::from_raw_parts_mut(root, crate::params::SPX_N) };
    let ap = unsafe { core::slice::from_raw_parts_mut(auth_path, (tree_height as usize) * crate::params::SPX_N) };
    let c = unsafe { &*ctx };
    let ta = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    let i = unsafe { &mut *info };
    crate::utilsx1::wots_treehashx1_internal(r, ap, c, leaf_idx, idx_offset, tree_height, ta, i);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_treehashx1(
    root: *mut u8, auth_path: *mut u8,
    ctx: *const crate::context::SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: *mut u32,
    info: *mut crate::context::ForsGenLeafInfo,
) {
    let r = unsafe { core::slice::from_raw_parts_mut(root, crate::params::SPX_N) };
    let ap = unsafe { core::slice::from_raw_parts_mut(auth_path, (tree_height as usize) * crate::params::SPX_N) };
    let c = unsafe { &*ctx };
    let ta = unsafe { &mut *(tree_addr as *mut [u32; 8]) };
    let i = unsafe { &mut *info };
    crate::utilsx1::fors_treehashx1_internal(r, ap, c, leaf_idx, idx_offset, tree_height, ta, i);
}

// compute_root and treehash exports
#[unsafe(no_mangle)]
pub extern "C" fn SPX_compute_root(
    root: *mut u8, leaf: *const u8,
    leaf_idx: u32, idx_offset: u32,
    auth_path: *const u8, tree_height: u32,
    ctx: *const crate::context::SpxCtx, addr: *mut u32,
) {
    let r = unsafe { core::slice::from_raw_parts_mut(root, crate::params::SPX_N) };
    let l = unsafe { core::slice::from_raw_parts(leaf, crate::params::SPX_N) };
    let ap = unsafe { core::slice::from_raw_parts(auth_path, (tree_height as usize) * crate::params::SPX_N) };
    let c = unsafe { &*ctx };
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    crate::utils::compute_root_internal(r, l, leaf_idx, idx_offset, ap, tree_height, c, a);
}
