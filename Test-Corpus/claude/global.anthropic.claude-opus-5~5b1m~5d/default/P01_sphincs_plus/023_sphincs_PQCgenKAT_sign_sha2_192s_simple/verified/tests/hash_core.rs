//! Phase B rows 17-28: `initialize_hash_function`, `prf_addr`, `thash` in
//! every `inblocks` shape, `gen_message_random`, `hash_message` and
//! `chain_lengths` -- all through both `.so`s.

mod common;
use common::*;
use libloading::Symbol;

type FThash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32);
type FPrf = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
type FGenMsgRand =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8);
type FHashMsg = unsafe extern "C" fn(
    *mut u8,   // digest
    *mut u64,  // tree
    *mut u32,  // leaf_idx
    *const u8, // R
    *const u8, // pk
    *const u8, // m
    u64,       // mlen
    *const u8, // ctx
);

// ---------------------------------------------------------------------------
// Row 17 -- initialize_hash_function: compare the WHOLE ctx byte image.
// This is the only way to check the sha2 precomputed midstates and the haraka
// tweaked round-constant tables.
// ---------------------------------------------------------------------------
#[test]
fn row17_initialize_hash_function() {
    let mut rng = Rng::new(RNG_SEED ^ 17);
    for iter in 0..N_ITER + 2 {
        let (ps, ss) = match iter {
            0 => (vec![0x00u8; SPX_N], vec![0x00u8; SPX_N]),
            1 => (vec![0xFFu8; SPX_N], vec![0xFFu8; SPX_N]),
            _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
        };
        let (cc, rc) = init_ctx_pair(&ps, &ss);
        eq(
            "sizeof(spx_ctx) implied by the harness",
            cc.len(),
            CTX_BYTES,
        );
        eq_bytes(
            &format!("initialize_hash_function ctx image (pub_seed={})", hex(&ps)),
            &cc,
            &rc,
        );
    }
}

/// `sizeof(spx_ctx)` is part of the FFI ABI: the C header only declares
/// `state_seeded_512` under `# if SPX_SHA512`, so sha2/128s and sha2/128f have a
/// 72-byte context, not 144. Give both libraries a context buffer with a marker
/// tail and check NEITHER writes past `CTX_BYTES` -- an over-sized Rust struct
/// would silently corrupt a real caller's memory and no output comparison would
/// notice.
#[test]
fn row17b_spx_ctx_size_is_abi_compatible() {
    let l = libs();
    let mut rng = Rng::new(RNG_SEED ^ 0x17B);
    const TAIL: usize = 256;
    for _ in 0..N_ITER {
        let ps = rng.bytes(SPX_N);
        let ss = rng.bytes(SPX_N);
        for (which, lib) in [("C", &l.c), ("Rust", &l.r)] {
            let mut buf = vec![0xA5u8; CTX_BYTES + TAIL];
            buf[..SPX_N].copy_from_slice(&ps);
            buf[SPX_N..2 * SPX_N].copy_from_slice(&ss);
            for b in buf[2 * SPX_N..].iter_mut() {
                *b = 0xA5;
            }
            let f: Symbol<unsafe extern "C" fn(*mut u8)> =
                sym(lib, "SPX_initialize_hash_function");
            unsafe { f(buf.as_mut_ptr()) };
            eq_bytes(
                &format!("{which}: initialize_hash_function wrote past sizeof(spx_ctx)={CTX_BYTES}"),
                &buf[CTX_BYTES..],
                &vec![0xA5u8; TAIL],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18 -- prf_addr
// ---------------------------------------------------------------------------
#[test]
fn row18_prf_addr() {
    let (c, r) = both!("SPX_prf_addr", FPrf);
    let mut rng = Rng::new(RNG_SEED ^ 18);
    for iter in 0..N_ITER + 2 {
        let (ps, ss) = match iter {
            0 => (vec![0x00u8; SPX_N], vec![0x00u8; SPX_N]),
            1 => (vec![0xFFu8; SPX_N], vec![0xFFu8; SPX_N]),
            _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
        };
        let (cctx, rctx) = init_ctx_pair(&ps, &ss);
        for addr in [[0u32; 8], [u32::MAX; 8], rng.addr(), rng.addr()] {
            // Overshoot the output buffer so any over-write is caught too.
            let mut co = vec![0xA5u8; SPX_N + 64];
            let mut ro = vec![0xA5u8; SPX_N + 64];
            let mut ca = addr;
            let mut ra = addr;
            unsafe {
                c(co.as_mut_ptr(), cctx.as_ptr(), ca.as_mut_ptr());
                r(ro.as_mut_ptr(), rctx.as_ptr(), ra.as_mut_ptr());
            }
            eq_bytes(&format!("prf_addr out (addr={addr:08X?})"), &co, &ro);
            eq("prf_addr must not modify addr", ca, ra);
            eq("prf_addr addr unchanged vs input", ca, addr);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 19-24 -- thash across every meaningful `inblocks`
// ---------------------------------------------------------------------------
fn thash_check(inblocks: u32, label: &str, seed: u64) {
    let (c, r) = both!("SPX_thash", FThash);
    let mut rng = Rng::new(seed);
    let in_len = (inblocks as usize) * SPX_N;
    for iter in 0..N_ITER {
        let (ps, ss) = match iter {
            0 => (vec![0x00u8; SPX_N], vec![0x00u8; SPX_N]),
            1 => (vec![0xFFu8; SPX_N], vec![0xFFu8; SPX_N]),
            _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
        };
        let (cctx, rctx) = init_ctx_pair(&ps, &ss);
        let inputs: Vec<Vec<u8>> = vec![
            vec![0x00u8; in_len.max(1)],
            vec![0xFFu8; in_len.max(1)],
            rng.bytes(in_len.max(1)),
        ];
        let addrs = [[0u32; 8], [u32::MAX; 8], rng.addr()];
        for inp in &inputs {
            for &addr in &addrs {
                let mut co = vec![0xA5u8; SPX_N + 64];
                let mut ro = vec![0xA5u8; SPX_N + 64];
                let mut ca = addr;
                let mut ra = addr;
                unsafe {
                    c(
                        co.as_mut_ptr(),
                        inp.as_ptr(),
                        inblocks,
                        cctx.as_ptr(),
                        ca.as_mut_ptr(),
                    );
                    r(
                        ro.as_mut_ptr(),
                        inp.as_ptr(),
                        inblocks,
                        rctx.as_ptr(),
                        ra.as_mut_ptr(),
                    );
                }
                eq_bytes(&format!("thash({label}) out"), &co, &ro);
                eq(&format!("thash({label}) addr side-effect"), ca, ra);
            }
        }
    }
}

#[test]
fn row19_thash_1_block() {
    thash_check(1, "inblocks=1", RNG_SEED ^ 19);
}

#[test]
fn row20_thash_2_blocks_switches_to_512() {
    // For sha2/blake at 192*/256* this is the inblocks>1 -> thash_512 switch.
    thash_check(2, "inblocks=2", RNG_SEED ^ 20);
}

#[test]
fn row21_thash_wots_len_blocks() {
    thash_check(
        SPX_WOTS_LEN as u32,
        &format!("inblocks=SPX_WOTS_LEN={SPX_WOTS_LEN}"),
        RNG_SEED ^ 21,
    );
}

#[test]
fn row22_thash_fors_trees_blocks() {
    thash_check(
        SPX_FORS_TREES as u32,
        &format!("inblocks=SPX_FORS_TREES={SPX_FORS_TREES}"),
        RNG_SEED ^ 22,
    );
}

#[test]
fn row23_thash_degenerate_and_odd() {
    thash_check(0, "inblocks=0", RNG_SEED ^ 23);
    thash_check(3, "inblocks=3", RNG_SEED ^ 231);
    thash_check(5, "inblocks=5", RNG_SEED ^ 232);
}

#[test]
fn row24_thash_value_extremes() {
    // Already covered inside thash_check (all-00 / all-FF inputs and seeds) for
    // inblocks 1 and 2; assert the extremes again explicitly with a fixed ctx
    // so a robust-vs-simple bitmask bug is unambiguous.
    let (c, r) = both!("SPX_thash", FThash);
    for &inblocks in &[1u32, 2, 3] {
        for ps in [vec![0x00u8; SPX_N], vec![0xFFu8; SPX_N]] {
            let (cctx, rctx) = init_ctx_pair(&ps, &ps);
            for fill in [0x00u8, 0xFF] {
                let inp = vec![fill; (inblocks as usize) * SPX_N];
                for &addr in &[[0u32; 8], [u32::MAX; 8]] {
                    let mut co = vec![0xA5u8; SPX_N + 64];
                    let mut ro = vec![0xA5u8; SPX_N + 64];
                    let mut ca = addr;
                    let mut ra = addr;
                    unsafe {
                        c(
                            co.as_mut_ptr(),
                            inp.as_ptr(),
                            inblocks,
                            cctx.as_ptr(),
                            ca.as_mut_ptr(),
                        );
                        r(
                            ro.as_mut_ptr(),
                            inp.as_ptr(),
                            inblocks,
                            rctx.as_ptr(),
                            ra.as_mut_ptr(),
                        );
                    }
                    eq_bytes(
                        &format!("thash extremes inblocks={inblocks} fill={fill:#02x}"),
                        &co,
                        &ro,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 -- gen_message_random over every message-length boundary
//
// NOTE: for the blake backend with SPX_N >= 24 the C writes 64 bytes into R
// (`blake512_final(&S, R)`) even though the caller only owns SPX_N. We give R a
// 64-byte tail and compare all of it, so the Rust must reproduce the same
// over-write byte for byte.
// ---------------------------------------------------------------------------
#[test]
fn row25_gen_message_random() {
    let (c, r) = both!("SPX_gen_message_random", FGenMsgRand);
    let mut rng = Rng::new(RNG_SEED ^ 25);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for &mlen in MLENS {
        for iter in 0..3 {
            let (sk_prf, optrand) = match iter {
                0 => (vec![0x00u8; SPX_N], vec![0x00u8; SPX_N]),
                1 => (vec![0xFFu8; SPX_N], vec![0xFFu8; SPX_N]),
                _ => (rng.bytes(SPX_N), rng.bytes(SPX_N)),
            };
            let m = rng.bytes(mlen.max(1));
            let mut cr = vec![0xA5u8; SPX_N + 64];
            let mut rr = vec![0xA5u8; SPX_N + 64];
            unsafe {
                c(
                    cr.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cctx.as_ptr(),
                );
                r(
                    rr.as_mut_ptr(),
                    sk_prf.as_ptr(),
                    optrand.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    rctx.as_ptr(),
                );
            }
            eq_bytes(&format!("gen_message_random(mlen={mlen}, iter={iter})"), &cr, &rr);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 26-27 -- hash_message (digest + tree + leaf_idx)
// ---------------------------------------------------------------------------
fn hash_message_check(rs: &[Vec<u8>], pks: &[Vec<u8>], seed: u64) {
    let (c, r) = both!("SPX_hash_message", FHashMsg);
    let mut rng = Rng::new(seed);
    let (cctx, rctx) = init_ctx_pair(&rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for &mlen in MLENS {
        let m = rng.bytes(mlen.max(1));
        for (rv, pk) in rs.iter().zip(pks.iter()) {
            let mut cd = vec![0xA5u8; SPX_FORS_MSG_BYTES + 32];
            let mut rd = vec![0xA5u8; SPX_FORS_MSG_BYTES + 32];
            let mut ct: u64 = 0xDEAD_BEEF_DEAD_BEEF;
            let mut rt: u64 = 0xDEAD_BEEF_DEAD_BEEF;
            let mut cl: u32 = 0xDEAD_BEEF;
            let mut rl: u32 = 0xDEAD_BEEF;
            unsafe {
                c(
                    cd.as_mut_ptr(),
                    &mut ct,
                    &mut cl,
                    rv.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    cctx.as_ptr(),
                );
                r(
                    rd.as_mut_ptr(),
                    &mut rt,
                    &mut rl,
                    rv.as_ptr(),
                    pk.as_ptr(),
                    m.as_ptr(),
                    mlen as u64,
                    rctx.as_ptr(),
                );
            }
            eq_bytes(&format!("hash_message digest (mlen={mlen})"), &cd, &rd);
            eq(&format!("hash_message *tree (mlen={mlen})"), ct, rt);
            eq(&format!("hash_message *leaf_idx (mlen={mlen})"), cl, rl);
        }
    }
}

#[test]
fn row26_hash_message_random() {
    let mut rng = Rng::new(RNG_SEED ^ 260);
    let rs: Vec<Vec<u8>> = (0..3).map(|_| rng.bytes(SPX_N)).collect();
    let pks: Vec<Vec<u8>> = (0..3).map(|_| rng.bytes(SPX_PK_BYTES)).collect();
    hash_message_check(&rs, &pks, RNG_SEED ^ 26);
}

#[test]
fn row27_hash_message_extremes() {
    let rs = vec![vec![0x00u8; SPX_N], vec![0xFFu8; SPX_N]];
    let pks = vec![vec![0x00u8; SPX_PK_BYTES], vec![0xFFu8; SPX_PK_BYTES]];
    hash_message_check(&rs, &pks, RNG_SEED ^ 27);
}

// ---------------------------------------------------------------------------
// Row 28 -- chain_lengths (base_w + WOTS checksum)
// ---------------------------------------------------------------------------
#[test]
fn row28_chain_lengths() {
    let (c, r) = both!(
        "SPX_chain_lengths",
        unsafe extern "C" fn(*mut u32, *const u8)
    );
    let mut rng = Rng::new(RNG_SEED ^ 28);
    let mut msgs: Vec<Vec<u8>> = vec![vec![0x00u8; SPX_N], vec![0xFFu8; SPX_N]];
    // Single-nibble sweeps: one byte at a time set to every nibble pattern.
    for v in [0x0Fu8, 0xF0, 0x01, 0x10, 0x88] {
        let mut m = vec![0u8; SPX_N];
        m[0] = v;
        m[SPX_N - 1] = v;
        msgs.push(m);
    }
    for _ in 0..N_ITER * 4 {
        msgs.push(rng.bytes(SPX_N));
    }
    for m in &msgs {
        let mut cl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
        let mut rl = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
        unsafe {
            c(cl.as_mut_ptr(), m.as_ptr());
            r(rl.as_mut_ptr(), m.as_ptr());
        }
        eq(&format!("chain_lengths(msg={})", hex(m)), &cl, &rl);
        for (i, &v) in cl[..SPX_WOTS_LEN].iter().enumerate() {
            assert!(
                (v as usize) < SPX_WOTS_W,
                "chain_lengths[{i}] = {v} >= SPX_WOTS_W (C invariant)"
            );
        }
    }
}
