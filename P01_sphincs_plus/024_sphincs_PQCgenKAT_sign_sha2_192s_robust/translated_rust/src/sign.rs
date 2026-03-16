use crate::context::SpxCtx;
use crate::params::*;
use crate::spx::*;

// WOTS

fn gen_chain(out: &mut [u8], inp: &[u8], start: u32, steps: u32, ctx: &SpxCtx, addr: &mut [u8; 32]) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);
    for i in start..start + steps {
        if i >= SPX_WOTS_W as u32 { break; }
        set_hash_addr(addr, i);
        let tmp = out[..SPX_N].to_vec();
        thash(out, &tmp, 1, ctx, addr);
    }
}

fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut inp_idx = 0;
    let mut bits = 0u32;
    let mut total = 0u8;
    for i in 0..out_len {
        if bits == 0 {
            total = input[inp_idx];
            inp_idx += 1;
            bits = 8;
        }
        bits -= SPX_WOTS_LOGW as u32;
        output[i] = ((total >> bits) & (SPX_WOTS_W as u8 - 1)) as u32;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum = 0u32;
    for i in 0..SPX_WOTS_LEN1 {
        csum += SPX_WOTS_W as u32 - 1 - msg_base_w[i];
    }
    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = [0u8; 4]; // max needed
    ull_to_bytes(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

pub fn chain_lengths(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let mut csum = [0u32; SPX_WOTS_LEN2];
    wots_checksum(&mut csum, lengths);
    lengths[SPX_WOTS_LEN1..].copy_from_slice(&csum);
}

pub fn wots_pk_from_sig(pk: &mut [u8], sig: &[u8], msg: &[u8], ctx: &SpxCtx, addr: &mut [u8; 32]) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);
    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr, i as u32);
        gen_chain(&mut pk[i * SPX_N..], &sig[i * SPX_N..], lengths[i], SPX_WOTS_W as u32 - 1 - lengths[i], ctx, addr);
    }
}

// WOTS x1

pub struct LeafInfoX1 {
    pub wots_sig: Vec<u8>,
    pub wots_sign_leaf: u32,
    pub wots_steps: [u32; SPX_WOTS_LEN],
    pub leaf_addr: [u8; 32],
    pub pk_addr: [u8; 32],
}

impl Default for LeafInfoX1 {
    fn default() -> Self {
        LeafInfoX1 {
            wots_sig: vec![0u8; SPX_WOTS_BYTES],
            wots_sign_leaf: 0,
            wots_steps: [0u32; SPX_WOTS_LEN],
            leaf_addr: [0u8; 32],
            pk_addr: [0u8; 32],
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

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(&mut pk_buffer[i * SPX_N..], ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                info.wots_sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let tmp = pk_buffer[i * SPX_N..(i + 1) * SPX_N].to_vec();
            thash(&mut pk_buffer[i * SPX_N..], &tmp, 1, ctx, &info.leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &info.pk_addr);
}

// FORS

pub struct ForsGenLeafInfo {
    pub leaf_addrx: [u8; 32],
}

impl Default for ForsGenLeafInfo {
    fn default() -> Self { ForsGenLeafInfo { leaf_addrx: [0u8; 32] } }
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, addr: &[u8; 32]) {
    prf_addr(sk, ctx, addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, addr: &[u8; 32]) {
    thash(leaf, sk, 1, ctx, addr);
}

pub fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    let mut tmp = [0u8; SPX_N];
    fors_gen_sk(&mut tmp, ctx, &info.leaf_addrx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    fors_sk_to_leaf(leaf, &tmp, ctx, &info.leaf_addrx);
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

pub fn fors_sign(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u8; 32]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u8; 32];
    let mut fors_info = ForsGenLeafInfo::default();
    let mut fors_pk_addr = [0u8; 32];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i].wrapping_add(idx_offset));
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        fors_gen_sk(&mut sig[sig_off..], ctx, &fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;

        // Use a LeafInfoX1 wrapper for fors_treehashx1 (it uses fors_gen_leafx1 internally)
        let mut leaf_info = LeafInfoX1::default();
        leaf_info.leaf_addr = fors_info.leaf_addrx;
        fors_treehashx1(&mut roots[i * SPX_N..], &mut sig[sig_off..], ctx,
                        indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
                        &mut fors_tree_addr, &mut fors_info);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &fors_pk_addr);
}

pub fn fors_pk_from_sig(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u8; 32]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = [0u8; 32];
    let mut fors_pk_addr = [0u8; 32];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);
        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i].wrapping_add(idx_offset));

        fors_sk_to_leaf(&mut leaf, &sig[sig_off..], ctx, &fors_tree_addr);
        sig_off += SPX_N;

        compute_root(&mut roots[i * SPX_N..], &leaf, indices[i], idx_offset,
                     &sig[sig_off..], SPX_FORS_HEIGHT as u32, ctx, &mut fors_tree_addr);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &fors_pk_addr);
}

// treehash x1 for WOTS
pub fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u8; 32], info: &mut LeafInfoX1) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx.wrapping_add(idx_offset), info);

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
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx { break; }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_start = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_start..left_start + SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let save_start = h as usize * SPX_N;
        stack[save_start..save_start + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

// treehash x1 for FORS
pub fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                       leaf_idx: u32, idx_offset: u32, tree_height: u32,
                       tree_addr: &mut [u8; 32], info: &mut ForsGenLeafInfo) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx.wrapping_add(idx_offset), info);

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
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx { break; }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let left_start = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[left_start..left_start + SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let save_start = h as usize * SPX_N;
        stack[save_start..save_start + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

// Merkle

pub fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
                   wots_addr: &mut [u8; 32], tree_addr: &mut [u8; 32], idx_leaf: u32) {
    let mut info = LeafInfoX1::default();
    let mut steps = [0u32; SPX_WOTS_LEN];

    info.wots_sig = vec![0u8; SPX_WOTS_BYTES];
    chain_lengths(&mut steps, root);
    info.wots_steps = steps;

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    info.wots_sign_leaf = idx_leaf;

    let auth_path_off = SPX_WOTS_BYTES;
    wots_treehashx1(root, &mut sig[auth_path_off..], ctx,
                    idx_leaf, 0, SPX_TREE_HEIGHT as u32,
                    tree_addr, &mut info);

    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig);
}

pub fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u8; 32];
    let mut wots_addr = [0u8; 32];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}
