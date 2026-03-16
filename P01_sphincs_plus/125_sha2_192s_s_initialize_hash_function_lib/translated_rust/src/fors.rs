use crate::context::{SpxCtx, ForsGenLeafInfo};
use crate::params::*;
use crate::utils::*;

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    crate::hash::prf_addr_internal(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    crate::thash::thash_internal(leaf, sk, 1, ctx, fors_leaf_addr);
}

fn message_to_indices(indices: &mut [u32], m: &[u8]) {
    let mut offset: usize = 0;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1) as u32) << j;
            offset += 1;
        }
    }
}

pub fn fors_sign_internal(
    sig: &mut [u8],
    pk: &mut [u8],
    m: &[u8],
    ctx: &SpxCtx,
    fors_addr: &[u32; 8],
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: [0u32; 8] };
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr_internal(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr_internal(&mut fors_info.leaf_addrx, fors_addr);
    copy_keypair_addr_internal(&mut fors_pk_addr, fors_addr);
    set_type_internal(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_offset = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height_internal(&mut fors_tree_addr, 0);
        set_tree_index_internal(&mut fors_tree_addr, indices[i].wrapping_add(idx_offset));
        set_type_internal(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        fors_gen_sk(&mut sig[sig_offset..], ctx, &fors_tree_addr);
        set_type_internal(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_offset += SPX_N;

        crate::utilsx1::fors_treehashx1_internal(
            &mut roots[i * SPX_N..],
            &mut sig[sig_offset..],
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_info,
        );
        sig_offset += SPX_N * SPX_FORS_HEIGHT;
    }

    crate::thash::thash_internal(pk, &roots, SPX_FORS_TREES as u32, ctx, &mut fors_pk_addr);
}

pub fn fors_pk_from_sig_internal(
    pk: &mut [u8],
    sig: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
    fors_addr: &[u32; 8],
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr_internal(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr_internal(&mut fors_pk_addr, fors_addr);
    set_type_internal(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type_internal(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_offset = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height_internal(&mut fors_tree_addr, 0);
        set_tree_index_internal(&mut fors_tree_addr, indices[i].wrapping_add(idx_offset));

        fors_sk_to_leaf(&mut leaf, &sig[sig_offset..], ctx, &mut fors_tree_addr);
        sig_offset += SPX_N;

        compute_root_internal(
            &mut roots[i * SPX_N..],
            &leaf,
            indices[i],
            idx_offset,
            &sig[sig_offset..],
            SPX_FORS_HEIGHT as u32,
            ctx,
            &mut fors_tree_addr,
        );
        sig_offset += SPX_N * SPX_FORS_HEIGHT;
    }

    crate::thash::thash_internal(pk, &roots, SPX_FORS_TREES as u32, ctx, &mut fors_pk_addr);
}
