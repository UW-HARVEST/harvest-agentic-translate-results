use crate::params::*;
use crate::context::SpxCtx;
use crate::address::*;
use crate::thash::thash;
use crate::hash::prf_addr;
use crate::wots::{LeafInfoX1, wots_gen_leafx1};

pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
}

impl Default for ForsGenLeafInfo {
    fn default() -> Self { ForsGenLeafInfo { leaf_addrx: [0u32; 8] } }
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let mut tmp = [0u8; SPX_N];
    tmp.copy_from_slice(&leaf[..SPX_N]);
    fors_sk_to_leaf(leaf, &tmp, ctx, &mut info.leaf_addrx);
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

pub fn fors_sign(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo::default();
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        fors_gen_sk(&mut sig[sig_off..], ctx, &fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;

        fors_treehashx1(&mut roots[i * SPX_N..], &mut sig[sig_off..], ctx,
                         indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
                         &mut fors_tree_addr, &mut fors_info);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

pub fn fors_pk_from_sig(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);
        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);

        fors_sk_to_leaf(&mut leaf, &sig[sig_off..], ctx, &mut fors_tree_addr);
        sig_off += SPX_N;

        crate::utils::compute_root(&mut roots[i * SPX_N..], &leaf, indices[i], idx_offset,
                                   &sig[sig_off..], SPX_FORS_HEIGHT as u32, ctx, &mut fors_tree_addr);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

// treehash for WOTS
pub fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
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
                let off = (h as usize) * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = (h as usize) * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save current to stack
        // h is the height where we stopped (the for loop variable)
        // We need to figure out h: it's the number of trailing 1-bits of idx, but
        // we can compute it from the break condition
        let mut h_val = 0u32;
        {
            let mut ti = idx;
            while ti & 1 == 1 {
                h_val += 1;
                ti >>= 1;
            }
        }
        // Actually, the C code saves at the h where the inner for loop broke.
        // Let's recompute: the inner loop runs h=0,1,... and breaks when (internal_idx & 1)==0 && idx < max_idx
        // At break, h is the current value. But we already modified internal_idx in the loop body.
        // The C code uses the h variable after the for loop. Let me re-examine.
        // In C: for(h=0;;h++, internal_idx>>=1, internal_leaf>>=1) { ... if break ... }
        // After break, h is the value at break. The increment happens at loop continuation.
        // So if we break at the check (internal_idx & 1)==0, h hasn't been incremented yet.
        // But internal_idx was NOT shifted yet (the shift is in the for increment).
        // Wait, in C the for loop is: for(h=0;; h++, internal_idx>>=1, internal_leaf>>=1)
        // The body runs, then h++, internal_idx>>=1, internal_leaf>>=1, then body again.
        // So at break, h is the current iteration's h value.
        // The stack save is: stack[h * SPX_N] = current[SPX_N]
        // h at break = number of trailing 1-bits of idx (since we break when internal_idx is even)
        // Actually: internal_idx starts as idx. After each iteration, it's shifted right.
        // We break when internal_idx & 1 == 0 (and idx < max_idx).
        // h=0: internal_idx = idx. If idx is even, break at h=0.
        // h=1: internal_idx = idx>>1 (shifted in for increment). If (idx>>1) is even, break at h=1.
        // So h = number of trailing 1-bits of idx.
        // But wait, the shifts happen in the for increment, which runs BEFORE the next iteration.
        // Actually no: for(init; cond; incr) { body }
        // Order: init, [cond, body, incr, cond, body, incr, ...]
        // So: h=0, body runs. If no break: h++, internal_idx>>=1, internal_leaf>>=1. Then body again.
        // At h=0: internal_idx = idx. Check: if idx&1==0 && idx<max_idx => break at h=0.
        // If idx is odd: do thash, then h becomes 1, internal_idx becomes idx>>1.
        // At h=1: check if (idx>>1)&1==0 => break at h=1.
        // So h = count of trailing 1-bits of idx. Correct.

        let off = (h_val as usize) * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

// treehash for FORS
pub fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
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
                let off = (h as usize) * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = (h as usize) * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let mut h_val = 0u32;
        { let mut ti = idx; while ti & 1 == 1 { h_val += 1; ti >>= 1; } }
        let off = (h_val as usize) * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}
