use crate::address::*;
use crate::context::SpxCtx;
use crate::haraka::*;
use crate::params::*;
use crate::wots::chain_lengths;

pub struct LeafInfoX1 {
    pub wots_sig: Vec<u8>,
    pub wots_sign_leaf: u32,
    pub wots_steps: [u32; SPX_WOTS_LEN],
    pub leaf_addr: [u32; 8],
    pub pk_addr: [u32; 8],
}

impl Default for LeafInfoX1 {
    fn default() -> Self {
        LeafInfoX1 {
            wots_sig: Vec::new(),
            wots_sign_leaf: 0,
            wots_steps: [0u32; SPX_WOTS_LEN],
            leaf_addr: [0u32; 8],
            pk_addr: [0u32; 8],
        }
    }
}

pub fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;
        let buf_start = i * SPX_N;

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);
        prf_addr(&mut pk_buffer[buf_start..], ctx, &info.leaf_addr);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                let src: Vec<u8> = pk_buffer[buf_start..buf_start + SPX_N].to_vec();
                info.wots_sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&src);
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let tmp: Vec<u8> = pk_buffer[buf_start..buf_start + SPX_N].to_vec();
            thash(&mut pk_buffer[buf_start..], &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &mut info.pk_addr);
}

// Generic treehash used by both wots and fors
fn treehashx1_generic(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    gen_leaf: &mut dyn FnMut(&mut [u8], &SpxCtx, u32),
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx: u32 = (1u32 << tree_height) - 1;

    let mut idx: u32 = 0;
    loop {
        let mut current = vec![0u8; 2 * SPX_N];
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
                let ap_off = h as usize * SPX_N;
                auth_path[ap_off..ap_off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..2 * SPX_N], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save current node to stack
        let off = h as usize * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);

        idx += 1;
    }
}

pub fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut LeafInfoX1,
) {
    // We need to pass info through the closure
    let info_ptr = info as *mut LeafInfoX1;
    treehashx1_generic(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        &mut |dest, ctx, leaf_idx_inner| {
            let info = unsafe { &mut *info_ptr };
            wots_gen_leafx1(dest, ctx, leaf_idx_inner, info);
        });
}

// FORS leaf generation
pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

impl Default for ForsGenLeafInfo {
    fn default() -> Self {
        ForsGenLeafInfo { leaf_addrx: [0u32; 8] }
    }
}

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    prf_addr(leaf, ctx, &info.leaf_addrx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let tmp: Vec<u8> = leaf[..SPX_N].to_vec();
    thash(leaf, &tmp, 1, ctx, &mut info.leaf_addrx);
}

pub fn fors_treehashx1(
    root: &mut [u8], auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut ForsGenLeafInfo,
) {
    let info_ptr = info as *mut ForsGenLeafInfo;
    treehashx1_generic(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        &mut |dest, ctx, addr_idx| {
            let info = unsafe { &mut *info_ptr };
            fors_gen_leafx1(dest, ctx, addr_idx, info);
        });
}
