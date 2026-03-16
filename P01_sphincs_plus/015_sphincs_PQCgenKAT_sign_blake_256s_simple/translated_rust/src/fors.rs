use crate::address::*;
use crate::hash::prf_addr;
use crate::params::*;
use crate::thash::thash;
use crate::wots::LeafInfoX1;

pub struct ForsGenLeafInfo {
    pub leaf_addrx: Addr,
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, addr: &Addr) {
    prf_addr(sk, ctx, addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, addr: &Addr) {
    thash(leaf, sk, 1, ctx, addr);
}

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let mut tmp = [0u8; SPX_N];
    tmp.copy_from_slice(&leaf[..SPX_N]);
    fors_sk_to_leaf(leaf, &tmp, ctx, &info.leaf_addrx);
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

pub fn fors_sign(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &Addr) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = addr_zero();
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: addr_zero() };
    let mut fors_pk_addr = addr_zero();

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

        fors_treehashx1(
            &mut roots[i * SPX_N..],
            &mut sig[sig_off..],
            ctx, indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr, &mut fors_info,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &fors_pk_addr);
}

pub fn fors_pk_from_sig(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &Addr) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = addr_zero();
    let mut fors_pk_addr = addr_zero();

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

        fors_sk_to_leaf(&mut leaf, &sig[sig_off..], ctx, &fors_tree_addr);
        sig_off += SPX_N;

        compute_root(
            &mut roots[i * SPX_N..], &leaf, indices[i], idx_offset,
            &sig[sig_off..], SPX_FORS_HEIGHT as u32, ctx, &mut fors_tree_addr,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &fors_pk_addr);
}

pub fn compute_root(
    root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
    auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut Addr,
) {
    let mut buffer = [0u8; 2 * SPX_N];
    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    }
    let mut auth_off = SPX_N;

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let mut tmp = [0u8; SPX_N];
            thash(&mut tmp, &buffer, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&tmp);
            buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        } else {
            let mut tmp = [0u8; SPX_N];
            thash(&mut tmp, &buffer, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&tmp);
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

/// Generic treehash used by both wots and fors variants
fn treehashx1_generic(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut Addr,
    gen_leaf: &mut dyn FnMut(&mut [u8], &SpxCtx, u32),
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        gen_leaf(&mut current[SPX_N..2 * SPX_N], ctx, idx + idx_offset);

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
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_off = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_off..left_off + SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
            h += 1;
        }

        stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

pub fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut Addr, info: &mut LeafInfoX1,
) {
    // We need to pass info mutably to the closure
    let info_ptr = info as *mut LeafInfoX1;
    treehashx1_generic(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        &mut |dest, ctx, leaf_idx_inner| {
            let info = unsafe { &mut *info_ptr };
            crate::wots::wots_gen_leafx1(dest, ctx, leaf_idx_inner, info);
        },
    );
}

pub fn fors_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut Addr, info: &mut ForsGenLeafInfo,
) {
    let info_ptr = info as *mut ForsGenLeafInfo;
    treehashx1_generic(root, auth_path, ctx, leaf_idx, idx_offset, tree_height, tree_addr,
        &mut |dest, ctx, addr_idx| {
            let info = unsafe { &mut *info_ptr };
            fors_gen_leafx1(dest, ctx, addr_idx, info);
        },
    );
}
