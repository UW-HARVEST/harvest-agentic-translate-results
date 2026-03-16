use crate::params::*;
use crate::address::*;
use crate::hash::SpxCtx;
use crate::hash::prf_addr_rs;
use crate::thash::thash_rs;
use crate::utils::compute_root_rs;
use crate::utilsx1::fors_treehashx1_rs;
use crate::wotsx1::LeafInfoX1;

#[repr(C)]
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    let ab = addr_as_bytes(fors_leaf_addr);
    prf_addr_rs(sk, ctx, ab);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    thash_rs(leaf, sk, 1, ctx, fors_leaf_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_gen_leafx1(
    leaf: *mut u8, ctx: *const SpxCtx, addr_idx: u32, info: *mut ForsGenLeafInfo,
) {
    let ctx = unsafe { &*ctx };
    let info = unsafe { &mut *info };
    let leaf = unsafe { std::slice::from_raw_parts_mut(leaf, SPX_N) };
    fors_gen_leafx1_rs(leaf, ctx, addr_idx, info);
}

pub fn fors_gen_leafx1_rs(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index_rs(&mut info.leaf_addrx, addr_idx);
    set_type_rs(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);
    set_type_rs(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let mut tmp = [0u8; SPX_N];
    tmp.copy_from_slice(&leaf[..SPX_N]);
    fors_sk_to_leaf(leaf, &tmp, ctx, &info.leaf_addrx);
}

fn message_to_indices(indices: &mut [u32], m: &[u8]) {
    let mut offset = 0usize;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1) as u32) << j;
            offset += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_sign(
    sig: *mut u8, pk: *mut u8, m: *const u8,
    ctx: *const SpxCtx, fors_addr: *const u32,
) {
    let ctx = unsafe { &*ctx };
    let fors_addr = unsafe { &*(fors_addr as *const [u32; 8]) };
    let m = unsafe { std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, SPX_FORS_BYTES) };
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_N) };
    fors_sign_rs(sig, pk, m, ctx, fors_addr);
}

pub fn fors_sign_rs(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: [0u32; 8] };
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr_rs(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr_rs(&mut fors_info.leaf_addrx, fors_addr);
    copy_keypair_addr_rs(&mut fors_pk_addr, fors_addr);
    set_type_rs(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_offset = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height_rs(&mut fors_tree_addr, 0);
        set_tree_index_rs(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type_rs(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        fors_gen_sk(&mut sig[sig_offset..], ctx, &fors_tree_addr);
        set_type_rs(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_offset += SPX_N;

        // Use LeafInfoX1 wrapper for fors_treehashx1
        let mut leaf_info = LeafInfoX1 {
            wots_sig: std::ptr::null_mut(),
            wots_sign_leaf: !0u32,
            wots_steps: std::ptr::null_mut(),
            leaf_addr: fors_info.leaf_addrx,
            pk_addr: [0u32; 8],
        };

        fors_treehashx1_rs(
            &mut roots[i * SPX_N..],
            &mut sig[sig_offset..],
            ctx, indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr, &mut leaf_info,
        );
        fors_info.leaf_addrx = leaf_info.leaf_addr;

        sig_offset += SPX_N * SPX_FORS_HEIGHT;
    }

    thash_rs(pk, &roots, SPX_FORS_TREES, ctx, &fors_pk_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_fors_pk_from_sig(
    pk: *mut u8, sig: *const u8, m: *const u8,
    ctx: *const SpxCtx, fors_addr: *const u32,
) {
    let ctx = unsafe { &*ctx };
    let fors_addr = unsafe { &*(fors_addr as *const [u32; 8]) };
    let m = unsafe { std::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let sig = unsafe { std::slice::from_raw_parts(sig, SPX_FORS_BYTES) };
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_N) };
    fors_pk_from_sig_rs(pk, sig, m, ctx, fors_addr);
}

pub fn fors_pk_from_sig_rs(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr_rs(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr_rs(&mut fors_pk_addr, fors_addr);
    set_type_rs(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type_rs(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_offset = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);
        set_tree_height_rs(&mut fors_tree_addr, 0);
        set_tree_index_rs(&mut fors_tree_addr, indices[i] + idx_offset);

        fors_sk_to_leaf(&mut leaf, &sig[sig_offset..], ctx, &fors_tree_addr);
        sig_offset += SPX_N;

        compute_root_rs(
            &mut roots[i * SPX_N..], &leaf, indices[i], idx_offset,
            &sig[sig_offset..], SPX_FORS_HEIGHT as u32, ctx, &mut fors_tree_addr,
        );
        sig_offset += SPX_N * SPX_FORS_HEIGHT;
    }

    thash_rs(pk, &roots, SPX_FORS_TREES, ctx, &fors_pk_addr);
}
