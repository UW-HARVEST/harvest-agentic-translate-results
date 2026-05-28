use crate::address::{copy_keypair_addr, set_tree_height, set_tree_index, set_type, SPX_ADDR_TYPE_FORSPK, SPX_ADDR_TYPE_FORSPRF, SPX_ADDR_TYPE_FORSTREE};
use crate::context::SpxCtx;
use crate::forsx1::ForsGenLeafInfo;
use crate::hash::prf_addr;
use crate::params::{SPX_FORS_BYTES, SPX_FORS_HEIGHT, SPX_FORS_MSG_BYTES, SPX_FORS_TREES, SPX_N};
use crate::thash::thash;
use crate::utils::compute_root;
use crate::utilsx1::fors_treehashx1;

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    let sk_copy = sk[..SPX_N].to_vec();
    thash(leaf, &sk_copy, 1, ctx, fors_leaf_addr);
}

fn message_to_indices(indices: &mut [u32], m: &[u8]) {
    let mut offset: usize = 0;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1u8) as u32) << j;
            offset += 1;
        }
    }
}

pub fn fors_sign(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = vec![0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: [0; 8] };
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_pos = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        // Include the secret key part that produces the selected leaf node.
        fors_gen_sk(&mut sig[sig_pos..sig_pos + SPX_N], ctx, &mut fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_pos += SPX_N;

        let (auth_path_slice, _) = sig[sig_pos..].split_at_mut(SPX_N * SPX_FORS_HEIGHT);
        fors_treehashx1(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
            auth_path_slice,
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_info,
        );

        sig_pos += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

pub fn fors_pk_from_sig(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = vec![0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = vec![0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);

    set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_pos = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);

        fors_sk_to_leaf(&mut leaf, &sig[sig_pos..sig_pos + SPX_N], ctx, &mut fors_tree_addr);
        sig_pos += SPX_N;

        compute_root(
            &mut roots[i * SPX_N..(i + 1) * SPX_N],
            &leaf,
            indices[i],
            idx_offset,
            &sig[sig_pos..],
            SPX_FORS_HEIGHT as u32,
            ctx,
            &mut fors_tree_addr,
        );
        sig_pos += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

// ---------- C-ABI exports ----------

#[unsafe(export_name = "SPX_fors_sign")]
pub unsafe extern "C" fn spx_fors_sign(
    sig: *mut u8,
    pk: *mut u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let sig_slice = unsafe { core::slice::from_raw_parts_mut(sig, SPX_FORS_BYTES) };
    let pk_slice = unsafe { core::slice::from_raw_parts_mut(pk, SPX_N) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let addr = unsafe { &*(fors_addr as *const [u32; 8]) };
    fors_sign(sig_slice, pk_slice, m_slice, unsafe { &*ctx }, addr);
}

#[unsafe(export_name = "SPX_fors_pk_from_sig")]
pub unsafe extern "C" fn spx_fors_pk_from_sig(
    pk: *mut u8,
    sig: *const u8,
    m: *const u8,
    ctx: *const SpxCtx,
    fors_addr: *const u32,
) {
    let pk_slice = unsafe { core::slice::from_raw_parts_mut(pk, SPX_N) };
    let sig_slice = unsafe { core::slice::from_raw_parts(sig, SPX_FORS_BYTES) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, SPX_FORS_MSG_BYTES) };
    let addr = unsafe { &*(fors_addr as *const [u32; 8]) };
    fors_pk_from_sig(pk_slice, sig_slice, m_slice, unsafe { &*ctx }, addr);
}
