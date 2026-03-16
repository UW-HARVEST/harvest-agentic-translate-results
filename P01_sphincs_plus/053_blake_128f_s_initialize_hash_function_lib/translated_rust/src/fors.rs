use crate::context::SpxCtx;
use crate::hash::*;
use crate::params::*;
use crate::utils::*;
use crate::wots::LeafInfoX1;

pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u32; 8],
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
    let mut offset: usize = 0;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1) as u32) << j;
            offset += 1;
        }
    }
}

pub fn compute_root(
    root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
    auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut [u32; 8],
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut auth_off: usize = 0;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    }
    auth_off += SPX_N;

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let buf_copy = buffer;
            thash(&mut buffer[SPX_N..], &buf_copy, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        } else {
            let buf_copy = buffer;
            thash(&mut buffer[..SPX_N], &buf_copy, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        }
        auth_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    thash(root, &buffer, 2, ctx, addr);
}

// treehash for wots
pub fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        crate::wots::wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let off = h as usize * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[off..off + SPX_N]);
            let current_copy = current;
            thash(&mut current[SPX_N..], &current_copy, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let h_val = {
            // find h where we broke out
            let mut h = 0u32;
            let mut ii = idx;
            loop {
                if h == tree_height { break; }
                if (ii ^ 0) == 0 { } // placeholder
                if (ii & 1) == 0 && idx < max_idx { break; }
                ii >>= 1; h += 1;
            }
            h
        };
        let off = h_val as usize * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
    }
}

// treehash for fors
pub fn fors_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: info.leaf_addr };

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, &mut fors_info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let off = h as usize * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[off..off + SPX_N]);
            let current_copy = current;
            thash(&mut current[SPX_N..], &current_copy, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let h_val = {
            let mut h = 0u32;
            let mut ii = idx;
            loop {
                if h == tree_height { break; }
                if (ii & 1) == 0 && idx < max_idx { break; }
                ii >>= 1; h += 1;
            }
            h
        };
        let off = h_val as usize * SPX_N;
        stack[off..off + SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
    }
}

pub fn fors_sign(
    sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8],
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: [0u32; 8] };
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off: usize = 0;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        fors_gen_sk(&mut sig[sig_off..], ctx, &fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;

        // Use a LeafInfoX1 wrapper for fors_treehashx1
        let mut leaf_info = LeafInfoX1 {
            wots_sig: vec![0u8; SPX_WOTS_BYTES],
            wots_sign_leaf: !0u32,
            wots_steps: vec![0u32; SPX_WOTS_LEN],
            leaf_addr: fors_info.leaf_addrx,
            pk_addr: [0u32; 8],
        };
        fors_treehashx1(
            &mut roots[i * SPX_N..],
            &mut sig[sig_off..],
            ctx, indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr, &mut leaf_info,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

pub fn fors_pk_from_sig(
    pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8],
) {
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

    let mut sig_off: usize = 0;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);

        fors_sk_to_leaf(&mut leaf, &sig[sig_off..], ctx, &mut fors_tree_addr);
        sig_off += SPX_N;

        compute_root(
            &mut roots[i * SPX_N..], &leaf, indices[i], idx_offset,
            &sig[sig_off..], SPX_FORS_HEIGHT as u32, ctx, &mut fors_tree_addr,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}
