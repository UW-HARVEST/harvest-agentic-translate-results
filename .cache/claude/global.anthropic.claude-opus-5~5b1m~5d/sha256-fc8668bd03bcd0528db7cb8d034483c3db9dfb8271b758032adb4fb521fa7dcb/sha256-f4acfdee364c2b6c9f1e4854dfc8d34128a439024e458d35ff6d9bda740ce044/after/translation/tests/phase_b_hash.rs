//! Phase B rows 13-18: gen_message_random and hash_message over every
//! message-length boundary the backends branch on.
mod common;
use common::*;

type FGmr = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8);
type FHm = unsafe extern "C" fn(*mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, u64, *const u8);

/// Every length at/around a rate or block boundary of any backend:
/// SHA-256 block 64, SHA-512 block 128, SHAKE-256 rate 136, haraka-S rate 32,
/// BLAKE-256 block 64, BLAKE-512 block 128, plus the `SPX_INBLOCKS` boundaries
/// of `hash_sha2.c` which depend on SPX_N and SPX_PK_BYTES.
fn boundary_lengths() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=40).collect();
    for base in [
        31usize, 32, 33, 47, 48, 49, 55, 56, 57, 63, 64, 65, 71, 72, 73, 79, 80, 81, 87, 88, 89,
        95, 96, 97, 103, 104, 105, 111, 112, 113, 119, 120, 121, 127, 128, 129, 135, 136, 137, 151,
        152, 153, 167, 168, 169, 175, 176, 177, 191, 192, 193, 199, 200, 201, 255, 256, 257, 271,
        272, 273, 383, 384, 385,
    ] {
        v.push(base);
    }
    // the exact sha2 branch boundaries
    let blk = if N >= 24 { 128usize } else { 64 };
    for d in [-2i64, -1, 0, 1, 2] {
        let a = blk as i64 - N as i64 + d;
        if a >= 0 {
            v.push(a as usize);
        }
        // SPX_INBLOCKS*blk - N - PK_BYTES
        let inblocks = (N + PK_BYTES + blk - 1) / blk;
        let b = (inblocks * blk) as i64 - N as i64 - PK_BYTES as i64 + d;
        if b >= 0 {
            v.push(b as usize);
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn b13_gen_message_random_empty() {
    let p = pair();
    p.check_config();
    let mut rng = Rng::new(SEED ^ 13);
    unsafe {
        let cf = sym!(p.c, b"SPX_gen_message_random\0", FGmr);
        let rf = sym!(p.r, b"SPX_gen_message_random\0", FGmr);
        for _ in 0..32 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            let sk_prf = rng.bytes(N);
            let optrand = rng.bytes(N);
            let m: Vec<u8> = vec![];
            let mut co = obuf(N);
            let mut ro = obuf(N);
            cf(co.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), m.as_ptr(), 0, cc.as_ptr());
            rf(ro.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), m.as_ptr(), 0, rc.as_ptr());
            eqb("gen_message_random mlen=0", &co, &ro);
        }
    }
}

#[test]
fn b14_gen_message_random_boundaries() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 14);
    unsafe {
        let cf = sym!(p.c, b"SPX_gen_message_random\0", FGmr);
        let rf = sym!(p.r, b"SPX_gen_message_random\0", FGmr);
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let cc = make_ctx(&p.c, &ps, &ss);
        let rc = make_ctx(&p.r, &ps, &ss);
        for mlen in boundary_lengths() {
            for _ in 0..3 {
                let sk_prf = rng.bytes(N);
                let optrand = rng.bytes(N);
                let m = rng.bytes(mlen.max(1));
                let mut co = obuf(N);
                let mut ro = obuf(N);
                cf(co.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), m.as_ptr(), mlen as u64, cc.as_ptr());
                rf(ro.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), m.as_ptr(), mlen as u64, rc.as_ptr());
                eqb(&format!("gen_message_random mlen={mlen}"), &co, &ro);
            }
        }
    }
}

#[test]
fn b15_gen_message_random_large() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 15);
    unsafe {
        let cf = sym!(p.c, b"SPX_gen_message_random\0", FGmr);
        let rf = sym!(p.r, b"SPX_gen_message_random\0", FGmr);
        for _ in 0..24 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            let mlen = rng.below(4096) as usize;
            let sk_prf = rng.bytes(N);
            let optrand = rng.bytes(N);
            let m = rng.bytes(mlen.max(1));
            let mut co = obuf(N);
            let mut ro = obuf(N);
            cf(co.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), m.as_ptr(), mlen as u64, cc.as_ptr());
            rf(ro.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), m.as_ptr(), mlen as u64, rc.as_ptr());
            eqb(&format!("gen_message_random large mlen={mlen}"), &co, &ro);
        }
    }
}

unsafe fn hm_case(p: &Pair, rng: &mut Rng, mlen: usize, iters: usize) {
    let cf = sym!(p.c, b"SPX_hash_message\0", FHm);
    let rf = sym!(p.r, b"SPX_hash_message\0", FHm);
    for _ in 0..iters {
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let cc = make_ctx(&p.c, &ps, &ss);
        let rc = make_ctx(&p.r, &ps, &ss);
        let r = rng.bytes(N);
        let pk = rng.bytes(PK_BYTES);
        let m = rng.bytes(mlen.max(1));

        let mut cd = obuf(FORS_MSG_BYTES);
        let mut rd = obuf(FORS_MSG_BYTES);
        let mut ct: u64 = 0xDEAD_BEEF_DEAD_BEEF;
        let mut rt: u64 = 0xDEAD_BEEF_DEAD_BEEF;
        let mut cl: u32 = 0xDEAD_BEEF;
        let mut rl: u32 = 0xDEAD_BEEF;

        cf(cd.as_mut_ptr(), &mut ct, &mut cl, r.as_ptr(), pk.as_ptr(), m.as_ptr(), mlen as u64, cc.as_ptr());
        rf(rd.as_mut_ptr(), &mut rt, &mut rl, r.as_ptr(), pk.as_ptr(), m.as_ptr(), mlen as u64, rc.as_ptr());
        eqb(&format!("hash_message digest mlen={mlen}"), &cd, &rd);
        eqv(&format!("hash_message tree mlen={mlen}"), ct, rt);
        eqv(&format!("hash_message leaf_idx mlen={mlen}"), cl, rl);
    }
}

#[test]
fn b16_hash_message_empty() {
    let p = pair();
    p.check_config();
    unsafe { hm_case(p, &mut Rng::new(SEED ^ 16), 0, 32) }
}

#[test]
fn b17_hash_message_boundaries() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 17);
    for mlen in boundary_lengths() {
        unsafe { hm_case(p, &mut rng, mlen, 2) }
    }
}

#[test]
fn b18_hash_message_large() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..24 {
        let mlen = rng.below(4096) as usize;
        unsafe { hm_case(p, &mut rng, mlen, 1) }
    }
}
