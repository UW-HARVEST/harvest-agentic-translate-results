use crate::params::*;
use crate::context::SpxCtx;
use crate::address::*;
use crate::hash::prf_addr;
use crate::thash::thash;
use crate::wots::chain_lengths;

pub struct LeafInfoX1 {
    pub wots_sig: Vec<u8>,
    pub wots_sign_leaf: u32,
    pub wots_steps: Vec<u32>,
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl Default for LeafInfoX1 {
    fn default() -> Self {
        LeafInfoX1 {
            wots_sig: Vec::new(),
            wots_sign_leaf: 0,
            wots_steps: Vec::new(),
            leaf_addr: [0; 8],
            pk_addr: [0; 8],
        }
    }
}

pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(buffer, ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                let sig_off = i * SPX_N;
                if sig_off + SPX_N <= info.wots_sig.len() {
                    info.wots_sig[sig_off..sig_off + SPX_N].copy_from_slice(buffer);
                }
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(buffer);
            thash(buffer, &tmp, 1, ctx, &info.leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &info.pk_addr);
}

// Generic treehash used by both wots and fors variants
fn treehashx1_generic(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8],
    gen_leaf: &mut dyn FnMut(&mut [u8], &SpxCtx, u32),
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx: u32 = (1u32 << tree_height) - 1;

    let mut idx: u32 = 0;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        gen_leaf(&mut current[SPX_N..2 * SPX_N], ctx, idx.wrapping_add(idx_offset));

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;

        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                let ho = h as usize;
                auth_path[ho * SPX_N..(ho + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            // Right node: combine with left from stack
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize;
            current[..SPX_N].copy_from_slice(&stack[ho * SPX_N..(ho + 1) * SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..2 * SPX_N], &tmp[..2 * SPX_N], 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save left child to stack
        let ho = h as usize;
        stack[ho * SPX_N..(ho + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

pub fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    // We need info to be mutably borrowed inside the closure
    // Use raw pointer trick to work around borrow checker
    let info_ptr = info as *mut LeafInfoX1;
    treehashx1_generic(
        root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        &mut |dest, ctx, leaf_idx_inner| {
            let info_ref = unsafe { &mut *info_ptr };
            wots_gen_leafx1(dest, ctx, leaf_idx_inner, info_ref);
        },
    );
}

pub fn fors_treehashx1(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut crate::fors::ForsGenLeafInfo,
) {
    let info_ptr = info as *mut crate::fors::ForsGenLeafInfo;
    treehashx1_generic(
        root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        &mut |dest, ctx, addr_idx| {
            let info_ref = unsafe { &mut *info_ptr };
            crate::fors::fors_gen_leafx1(dest, ctx, addr_idx, info_ref);
        },
    );
}
