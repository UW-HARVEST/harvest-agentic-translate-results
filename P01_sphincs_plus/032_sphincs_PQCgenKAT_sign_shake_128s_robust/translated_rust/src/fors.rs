use crate::params::*;
use crate::context::SpxCtx;
use crate::address;
use crate::thash;
use crate::hash;
use crate::wotsx1::{LeafInfoX1, wots_gen_leafx1};

pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    address::set_tree_index(&mut info.leaf_addrx, addr_idx);
    address::set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    hash::prf_addr(leaf, ctx, &info.leaf_addrx);
    address::set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let mut tmp = [0u8; SPX_N];
    tmp.copy_from_slice(&leaf[..SPX_N]);
    thash::thash(leaf, &tmp, 1, ctx, &mut info.leaf_addrx);
}

fn message_to_indices(indices: &mut [u32; SPX_FORS_TREES], m: &[u8]) {
    let mut offset = 0usize;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1) as u32) << j;
            offset += 1;
        }
    }
}

pub fn fors_sign(sig: &mut [u8], pk: &mut [u8], m: &[u8],
                 ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: [0u32; 8] };
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

        hash::prf_addr(&mut sig[sig_off..], ctx, &fors_tree_addr);
        address::set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;

        // Use fors_treehashx1
        fors_treehashx1(&mut roots[i * SPX_N..], &mut sig[sig_off..], ctx,
                        indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
                        &mut fors_tree_addr, &mut fors_info);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash::thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

pub fn fors_pk_from_sig(pk: &mut [u8], sig: &[u8], m: &[u8],
                        ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
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

        // sk_to_leaf
        thash::thash(&mut leaf, &sig[sig_off..], 1, ctx, &mut fors_tree_addr);
        sig_off += SPX_N;

        crate::utils::compute_root(&mut roots[i * SPX_N..], &leaf, indices[i], idx_offset,
                                   &sig[sig_off..], SPX_FORS_HEIGHT as u32, ctx, &mut fors_tree_addr);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash::thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

// fors_treehashx1 - same algorithm as wots_treehashx1 but uses fors_gen_leafx1
fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                   leaf_idx: u32, idx_offset: u32, tree_height: u32,
                   tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                // save and break
                stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
                break;
            }
            internal_idx_offset >>= 1;
            address::set_tree_height(tree_addr, h + 1);
            address::set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..SPX_N].copy_from_slice(&stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]);
            let tmp: Vec<u8> = current[..2 * SPX_N].to_vec();
            thash::thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
    }
}

// wots_treehashx1
pub fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
                break;
            }
            internal_idx_offset >>= 1;
            address::set_tree_height(tree_addr, h + 1);
            address::set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..SPX_N].copy_from_slice(&stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]);
            let tmp: Vec<u8> = current[..2 * SPX_N].to_vec();
            thash::thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
    }
}
