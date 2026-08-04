use crate::address::{copy_keypair_addr_rs, set_tree_height_rs, set_tree_index_rs, set_type_rs};
use crate::context::spx_ctx;
use crate::params::*;
use crate::sha2_backend::{SPX_prf_addr_rs, SPX_thash_rs};
use crate::utils::compute_root_rs;
use crate::utilsx1::fors_treehashx1_rs;

#[repr(C)]
pub struct fors_gen_leaf_info {
    pub leaf_addrx: [u32; 8],
}

fn fors_gen_sk(sk: &mut [u8], ctx: &spx_ctx, fors_leaf_addr: &[u32; 8]) {
    SPX_prf_addr_rs(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &spx_ctx, fors_leaf_addr: &mut [u32; 8]) {
    SPX_thash_rs(leaf, sk, 1, ctx, fors_leaf_addr);
}

pub(crate) fn fors_gen_leafx1_rs(leaf: &mut [u8], ctx: &spx_ctx, addr_idx: u32, info: &mut fors_gen_leaf_info) {
    set_tree_index_rs(&mut info.leaf_addrx, addr_idx);
    set_type_rs(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);
    set_type_rs(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let tmp = leaf.to_vec();
    fors_sk_to_leaf(leaf, &tmp, ctx, &mut info.leaf_addrx);
}

fn message_to_indices(indices: &mut [u32], m: &[u8]) {
    let mut offset = 0usize;
    for idx in indices.iter_mut().take(SPX_FORS_TREES) {
        *idx = 0;
        for j in 0..SPX_FORS_HEIGHT {
            *idx ^= (((m[offset >> 3] >> (offset & 7)) & 1) as u32) << j;
            offset += 1;
        }
    }
}

pub(crate) fn fors_sign_rs(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &spx_ctx, fors_addr: &[u32; 8]) {
    let mut indices = vec![0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = fors_gen_leaf_info { leaf_addrx: [0u32; 8] };
    let mut fors_pk_addr = [0u32; 8];
    copy_keypair_addr_rs(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr_rs(&mut fors_info.leaf_addrx, fors_addr);
    copy_keypair_addr_rs(&mut fors_pk_addr, fors_addr);
    set_type_rs(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);
    message_to_indices(&mut indices, m);
    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);
        set_tree_height_rs(&mut fors_tree_addr, 0);
        set_tree_index_rs(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type_rs(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);
        fors_gen_sk(&mut sig[sig_off..sig_off + SPX_N], ctx, &fors_tree_addr);
        set_type_rs(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;
        fors_treehashx1_rs(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
            &mut sig[sig_off..sig_off + SPX_N * SPX_FORS_HEIGHT],
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_info,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }
    SPX_thash_rs(pk, &roots, SPX_FORS_TREES as u32, ctx, &mut fors_pk_addr);
}

pub(crate) fn fors_pk_from_sig_rs(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &spx_ctx, fors_addr: &[u32; 8]) {
    let mut indices = vec![0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];
    copy_keypair_addr_rs(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr_rs(&mut fors_pk_addr, fors_addr);
    set_type_rs(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type_rs(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);
    message_to_indices(&mut indices, m);
    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);
        set_tree_height_rs(&mut fors_tree_addr, 0);
        set_tree_index_rs(&mut fors_tree_addr, indices[i] + idx_offset);
        fors_sk_to_leaf(&mut leaf, &sig[sig_off..sig_off + SPX_N], ctx, &mut fors_tree_addr);
        sig_off += SPX_N;
        compute_root_rs(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
            &leaf,
            indices[i],
            idx_offset,
            &sig[sig_off..sig_off + SPX_N * SPX_FORS_HEIGHT],
            SPX_FORS_HEIGHT as u32,
            ctx,
            &mut fors_tree_addr,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }
    SPX_thash_rs(pk, &roots, SPX_FORS_TREES as u32, ctx, &mut fors_pk_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_sign(sig: *mut u8, pk: *mut u8, m: *const u8, ctx: *const spx_ctx, fors_addr: *const u32) {
    fors_sign_rs(
        unsafe { std::slice::from_raw_parts_mut(sig, SPX_FORS_BYTES) },
        unsafe { std::slice::from_raw_parts_mut(pk, SPX_N) },
        unsafe { std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) },
        unsafe { &*ctx },
        unsafe { &*(fors_addr as *const [u32; 8]) },
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_pk_from_sig(pk: *mut u8, sig: *const u8, m: *const u8, ctx: *const spx_ctx, fors_addr: *const u32) {
    fors_pk_from_sig_rs(
        unsafe { std::slice::from_raw_parts_mut(pk, SPX_N) },
        unsafe { std::slice::from_raw_parts(sig, SPX_FORS_BYTES) },
        unsafe { std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) },
        unsafe { &*ctx },
        unsafe { &*(fors_addr as *const [u32; 8]) },
    );
}
