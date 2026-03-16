use crate::context::SpxCtx;
use crate::params::*;
use crate::address::*;
use crate::hash::prf_addr;
use crate::thash::thash;
use crate::wots::chain_lengths;
use crate::fors::fors_gen_leafx1;

pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32,
                       wots_sig: &mut [u8], wots_sign_leaf: u32, wots_steps: &[u32],
                       leaf_addr: &mut [u8; 32], pk_addr: &mut [u8; 32]) {
    let wots_k_mask: u32 = if leaf_idx == wots_sign_leaf { 0 } else { !0u32 };
    set_keypair_addr(leaf_addr, leaf_idx);
    set_keypair_addr(pk_addr, leaf_idx);
    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];
    for i in 0..SPX_WOTS_LEN {
        let wots_k = wots_steps[i] | wots_k_mask;
        set_chain_addr(leaf_addr, i as u32);
        set_hash_addr(leaf_addr, 0);
        set_type(leaf_addr, SPX_ADDR_TYPE_WOTSPRF);
        prf_addr(&mut pk_buffer[i * SPX_N..], ctx, leaf_addr);
        set_type(leaf_addr, SPX_ADDR_TYPE_WOTS);
        for k in 0u32.. {
            if k == wots_k {
                wots_sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(leaf_addr, k);
            let tmp: Vec<u8> = pk_buffer[i * SPX_N..(i + 1) * SPX_N].to_vec();
            thash(&mut pk_buffer[i * SPX_N..], &tmp, 1, ctx, leaf_addr);
        }
    }
    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, pk_addr);
}

pub fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u8; 32],
                       wots_sig: &mut [u8], wots_sign_leaf: u32, wots_steps: &[u32],
                       leaf_addr: &mut [u8; 32], pk_addr: &mut [u8; 32]) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset,
                        wots_sig, wots_sign_leaf, wots_steps, leaf_addr, pk_addr);
        let mut iio = idx_offset;
        let mut ii = idx;
        let mut il = leaf_idx;
        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (ii ^ il) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (ii & 1) == 0 && idx < max_idx { break; }
            iio >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, ii / 2 + iio);
            current[..SPX_N].copy_from_slice(&stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]);
            let tmp: Vec<u8> = current[..2 * SPX_N].to_vec();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);
            h += 1; ii >>= 1; il >>= 1;
        }
        stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

pub fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u8; 32], fors_leaf_addr: &mut [u8; 32],
                       _is_fors: bool) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, fors_leaf_addr);
        let mut iio = idx_offset;
        let mut ii = idx;
        let mut il = leaf_idx;
        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (ii ^ il) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (ii & 1) == 0 && idx < max_idx { break; }
            iio >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, ii / 2 + iio);
            current[..SPX_N].copy_from_slice(&stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]);
            let tmp: Vec<u8> = current[..2 * SPX_N].to_vec();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);
            h += 1; ii >>= 1; il >>= 1;
        }
        stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}
