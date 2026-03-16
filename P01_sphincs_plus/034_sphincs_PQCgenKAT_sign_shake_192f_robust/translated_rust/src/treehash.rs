#![allow(unused_imports, unused_variables, unused_assignments)]
use crate::params::*;
use crate::context::*;
use crate::hash::*;
use crate::wots::chain_lengths;

pub struct LeafInfoX1 {
    pub wots_sig: Vec<u8>,
    pub wots_sign_leaf: u32,
    pub wots_steps: [u32; SPX_WOTS_LEN],
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;
        let buf = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(buf, ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                info.wots_sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(buf);
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(buf);
            thash(buf, &tmp, 1, ctx, &info.leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &info.pk_addr);
}

pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    prf_addr(sk, ctx, addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    thash(leaf, sk, 1, ctx, addr);
}

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let tmp: Vec<u8> = leaf[..SPX_N].to_vec();
    fors_sk_to_leaf(leaf, &tmp, ctx, &info.leaf_addrx);
}

// treehash for wots
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
                let ap_off = h as usize * SPX_N;
                auth_path[ap_off..ap_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let s_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[s_off..s_off + SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // find h from the loop - we need to know where we broke
        let mut h = 0u32;
        {
            let mut ti = idx;
            let mut tl = leaf_idx;
            let mut tio = idx_offset;
            loop {
                if h == tree_height { break; }
                if (ti ^ tl) == 0x01 { }
                if (ti & 1) == 0 && idx < max_idx { break; }
                tio >>= 1;
                ti >>= 1;
                tl >>= 1;
                h += 1;
            }
        }
        let s_off = h as usize * SPX_N;
        stack[s_off..s_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

// treehash for fors
pub fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
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
                let ap_off = h as usize * SPX_N;
                auth_path[ap_off..ap_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let s_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[s_off..s_off + SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let mut h = 0u32;
        {
            let mut ti = idx;
            let mut tl = leaf_idx;
            loop {
                if h == tree_height { break; }
                if (ti & 1) == 0 && idx < max_idx { break; }
                ti >>= 1;
                tl >>= 1;
                h += 1;
            }
        }
        let s_off = h as usize * SPX_N;
        stack[s_off..s_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}
