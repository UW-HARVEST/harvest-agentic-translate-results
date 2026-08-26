use crate::address;
use crate::context::SpxCtx;
use crate::hash::prf_addr;
use crate::params::{
    SPX_ADDR_TYPE_FORSPK, SPX_ADDR_TYPE_FORSPRF, SPX_ADDR_TYPE_FORSTREE, SPX_FORS_HEIGHT,
    SPX_FORS_TREES, SPX_N,
};
use crate::thash::thash;
use crate::utils::compute_root;
use crate::utilsx1::{fors_treehashx1, ForsGenLeafInfo};

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    address::set_tree_index(&mut info.leaf_addrx, addr_idx);
    address::set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);

    address::set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let leaf_clone = leaf[..SPX_N].to_vec();
    fors_sk_to_leaf(&mut leaf[..SPX_N], &leaf_clone, ctx, &mut info.leaf_addrx);
}

fn message_to_indices(indices: &mut [u32], m: &[u8]) {
    let mut offset = 0usize;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1u8) as u32) << j;
            offset += 1;
        }
    }
}

/// Sign a message using FORS.
pub fn fors_sign(
    sig_buf: &mut [u8],
    pk: &mut [u8],
    m: &[u8],
    ctx: &SpxCtx,
    fors_addr: &[u32; 8],
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo {
        leaf_addrx: [0u32; 8],
    };
    let mut fors_pk_addr = [0u32; 8];

    address::copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    address::copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr);

    address::copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    address::set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        address::set_tree_height(&mut fors_tree_addr, 0);
        address::set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);
        address::set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        fors_gen_sk(&mut sig_buf[sig_off..sig_off + SPX_N], ctx, &fors_tree_addr);
        address::set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;

        // Compute the auth path for this leaf.
        // sig_buf[sig_off..sig_off + SPX_N * SPX_FORS_HEIGHT] is the auth path
        let auth_len = SPX_N * SPX_FORS_HEIGHT;
        let (auth_path, _) = sig_buf[sig_off..sig_off + auth_len].split_at_mut(auth_len);

        // We need a separate root buffer for this tree.
        let mut root_tmp = vec![0u8; SPX_N];
        fors_treehashx1(
            &mut root_tmp,
            auth_path,
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_info,
        );
        roots[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&root_tmp);

        sig_off += auth_len;
    }

    thash(pk, &roots, SPX_FORS_TREES as u32, ctx, &mut fors_pk_addr);
}

/// Derive the FORS public key from a signature.
pub fn fors_pk_from_sig(
    pk: &mut [u8],
    sig_buf: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
    fors_addr: &[u32; 8],
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    address::copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    address::copy_keypair_addr(&mut fors_pk_addr, fors_addr);

    address::set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    address::set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        address::set_tree_height(&mut fors_tree_addr, 0);
        address::set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);

        // Derive leaf from secret-key part included in the signature.
        fors_sk_to_leaf(
            &mut leaf,
            &sig_buf[sig_off..sig_off + SPX_N],
            ctx,
            &mut fors_tree_addr,
        );
        sig_off += SPX_N;

        // Derive corresponding root node of this tree.
        let auth_len = SPX_N * SPX_FORS_HEIGHT;
        let auth_path = &sig_buf[sig_off..sig_off + auth_len];
        let mut root_tmp = vec![0u8; SPX_N];
        compute_root(
            &mut root_tmp,
            &leaf,
            indices[i],
            idx_offset,
            auth_path,
            SPX_FORS_HEIGHT as u32,
            ctx,
            &mut fors_tree_addr,
        );
        roots[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&root_tmp);
        sig_off += auth_len;
    }

    thash(pk, &roots, SPX_FORS_TREES as u32, ctx, &mut fors_pk_addr);
}
