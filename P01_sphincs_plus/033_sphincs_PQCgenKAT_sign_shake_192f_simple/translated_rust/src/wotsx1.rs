use crate::params::*;
use crate::address::*;
use crate::hash::*;

// wotsx1.c: wots_gen_leafx1
pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32,
                       wots_sig: &mut [u8], wots_sign_leaf: u32,
                       wots_steps: &[u32; SPX_WOTS_LEN],
                       leaf_addr: &mut Addr, pk_addr: &mut Addr) {
    let wots_k_mask: u32 = if leaf_idx == wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(leaf_addr, leaf_idx);
    set_keypair_addr(pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = wots_steps[i] | wots_k_mask;
        let buf = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        set_chain_addr(leaf_addr, i as u32);
        set_hash_addr(leaf_addr, 0);
        set_type(leaf_addr, SPX_ADDR_TYPE_WOTSPRF);
        prf_addr(buf, ctx, leaf_addr);
        set_type(leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                wots_sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(buf);
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            thash(&mut tmp, buf, 1, ctx, leaf_addr);
            buf.copy_from_slice(&tmp);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, pk_addr);
}

// utilsx1.c: wots_treehashx1
pub fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut Addr,
                       wots_sig: &mut [u8], wots_sign_leaf: u32,
                       wots_steps: &[u32; SPX_WOTS_LEN],
                       leaf_addr: &mut Addr, pk_addr: &mut Addr) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset,
                        wots_sig, wots_sign_leaf, wots_steps,
                        leaf_addr, pk_addr);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        let mut h = 0u32;
        loop {
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
            let mut tmp = [0u8; SPX_N];
            thash(&mut tmp, &current, 2, ctx, tree_addr);
            current[SPX_N..2 * SPX_N].copy_from_slice(&tmp);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let s_off = h as usize * SPX_N;
        stack[s_off..s_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}
