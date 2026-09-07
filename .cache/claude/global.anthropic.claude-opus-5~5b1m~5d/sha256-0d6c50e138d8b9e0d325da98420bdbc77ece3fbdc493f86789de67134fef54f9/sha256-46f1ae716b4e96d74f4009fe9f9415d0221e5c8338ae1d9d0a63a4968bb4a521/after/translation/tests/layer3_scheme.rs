//! Phase B rows 44-47: `fors.c` (`fors_sign`, `fors_pk_from_sig`) and
//! `merkle.c` (`merkle_sign`, `merkle_gen_root`).
mod common;
use common::*;

type FnInitHash = unsafe extern "C" fn(*mut SpxCtxFfi);
type FnForsSign =
    unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const SpxCtxFfi, *const u32);
type FnForsPkFromSig =
    unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const SpxCtxFfi, *const u32);
type FnMerkleSign =
    unsafe extern "C" fn(*mut u8, *mut u8, *const SpxCtxFfi, *mut u32, *mut u32, u32);
type FnMerkleGenRoot = unsafe extern "C" fn(*mut u8, *const SpxCtxFfi);

fn make_ctx(rng: &mut Rng) -> (SpxCtxFfi, SpxCtxFfi) {
    let l = libs();
    let (c, r) = l.pair::<FnInitHash>("SPX_initialize_hash_function");
    let mut cc = SpxCtxFfi::zeroed();
    rng.fill(&mut cc.pub_seed);
    rng.fill(&mut cc.sk_seed);
    let mut rc = cc.clone();
    unsafe {
        c(&mut cc);
        r(&mut rc);
    }
    assert_bytes_eq("spx_ctx after init", cc.as_bytes(), rc.as_bytes());
    (cc, rc)
}

// --- row 44 -------------------------------------------------------------

#[test]
fn row44_fors_sign() {
    let l = libs();
    let (c, r) = l.pair::<FnForsSign>("SPX_fors_sign");
    let mut rng = Rng::new(0x4401);
    for it in 0..24 {
        let (cc, rc) = make_ctx(&mut rng);
        let m = match it {
            0 => vec![0x00u8; SPX_FORS_MSG_BYTES],
            1 => vec![0xFFu8; SPX_FORS_MSG_BYTES],
            2 => vec![0xAAu8; SPX_FORS_MSG_BYTES],
            3 => vec![0x55u8; SPX_FORS_MSG_BYTES],
            _ => rng.bytes(SPX_FORS_MSG_BYTES),
        };
        let fors_addr = rng.addr();
        let mut csig = vec![0xA5u8; SPX_FORS_BYTES + 8];
        let mut rsig = vec![0xA5u8; SPX_FORS_BYTES + 8];
        let mut cpk = vec![0xA5u8; SPX_N + 8];
        let mut rpk = vec![0xA5u8; SPX_N + 8];
        unsafe {
            c(csig.as_mut_ptr(), cpk.as_mut_ptr(), m.as_ptr(), &cc, fors_addr.as_ptr());
            r(rsig.as_mut_ptr(), rpk.as_mut_ptr(), m.as_ptr(), &rc, fors_addr.as_ptr());
        }
        assert_bytes_eq(&format!("SPX_fors_sign #{it} sig"), &csig, &rsig);
        assert_bytes_eq(&format!("SPX_fors_sign #{it} pk"), &cpk, &rpk);
        // fors_addr is `const uint32_t[8]` -- must not be modified.
        assert_bytes_eq("SPX_fors_sign ctx unchanged", cc.as_bytes(), rc.as_bytes());
    }
}

// --- row 45 -------------------------------------------------------------

#[test]
fn row45_fors_pk_from_sig() {
    let l = libs();
    let (cs, rs) = l.pair::<FnForsSign>("SPX_fors_sign");
    let (cv, rv) = l.pair::<FnForsPkFromSig>("SPX_fors_pk_from_sig");
    let mut rng = Rng::new(0x4501);
    for it in 0..24 {
        let (cc, rc) = make_ctx(&mut rng);
        let m = match it {
            0 => vec![0x00u8; SPX_FORS_MSG_BYTES],
            1 => vec![0xFFu8; SPX_FORS_MSG_BYTES],
            _ => rng.bytes(SPX_FORS_MSG_BYTES),
        };
        let fors_addr = rng.addr();

        // (a) round trip: sign then recover -- must give back the same pk.
        let mut sig = vec![0u8; SPX_FORS_BYTES];
        let mut pk0 = vec![0u8; SPX_N];
        unsafe {
            cs(sig.as_mut_ptr(), pk0.as_mut_ptr(), m.as_ptr(), &cc, fors_addr.as_ptr());
        }
        let mut cpk = vec![0xA5u8; SPX_N + 8];
        let mut rpk = vec![0xA5u8; SPX_N + 8];
        unsafe {
            cv(cpk.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), &cc, fors_addr.as_ptr());
            rv(rpk.as_mut_ptr(), sig.as_ptr(), m.as_ptr(), &rc, fors_addr.as_ptr());
        }
        assert_bytes_eq(&format!("SPX_fors_pk_from_sig #{it} (round trip)"), &cpk, &rpk);
        assert_bytes_eq(
            &format!("SPX_fors_pk_from_sig #{it} recovers fors_sign's pk (C)"),
            &pk0,
            &cpk[..SPX_N],
        );
        // Rust's own fors_sign must produce the identical sig too.
        let mut rsig = vec![0u8; SPX_FORS_BYTES];
        let mut rpk0 = vec![0u8; SPX_N];
        unsafe {
            rs(rsig.as_mut_ptr(), rpk0.as_mut_ptr(), m.as_ptr(), &rc, fors_addr.as_ptr());
        }
        assert_bytes_eq(&format!("SPX_fors_sign #{it} sig (cross-check)"), &sig, &rsig);

        // (b) GARBAGE signature: both sides must still agree byte-for-byte.
        let garbage = rng.bytes(SPX_FORS_BYTES);
        let mut cg = vec![0xA5u8; SPX_N + 8];
        let mut rg = vec![0xA5u8; SPX_N + 8];
        unsafe {
            cv(cg.as_mut_ptr(), garbage.as_ptr(), m.as_ptr(), &cc, fors_addr.as_ptr());
            rv(rg.as_mut_ptr(), garbage.as_ptr(), m.as_ptr(), &rc, fors_addr.as_ptr());
        }
        assert_bytes_eq(&format!("SPX_fors_pk_from_sig #{it} (garbage sig)"), &cg, &rg);
    }
}

// --- row 46 -------------------------------------------------------------

#[test]
fn row46_merkle_sign() {
    let l = libs();
    let (c, r) = l.pair::<FnMerkleSign>("SPX_merkle_sign");
    let mut rng = Rng::new(0x4601);
    let siglen = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
    let max = (1u32 << SPX_TREE_HEIGHT) - 1;
    for it in 0..24 {
        let (cc, rc) = make_ctx(&mut rng);
        let idx_leaf = match it {
            0 => 0u32,
            1 => 1,
            2 => max,
            3 => u32::MAX, // merkle_gen_root's "no auth path" sentinel
            _ => rng.next_u32() & max,
        };
        // `root` is IN/OUT: on entry it is the message the WOTS key signs, on
        // exit it is the subtree root.
        let root_in = rng.bytes(SPX_N);
        let wots_addr = rng.addr();
        let tree_addr = rng.addr();

        let mut cwa = wots_addr;
        let mut rwa = wots_addr;
        let mut cta = tree_addr;
        let mut rta = tree_addr;
        let mut csig = vec![0xA5u8; siglen + 8];
        let mut rsig = vec![0xA5u8; siglen + 8];
        let mut croot = vec![0xA5u8; SPX_N + 8];
        let mut rroot = vec![0xA5u8; SPX_N + 8];
        croot[..SPX_N].copy_from_slice(&root_in);
        rroot[..SPX_N].copy_from_slice(&root_in);
        unsafe {
            c(
                csig.as_mut_ptr(),
                croot.as_mut_ptr(),
                &cc,
                cwa.as_mut_ptr(),
                cta.as_mut_ptr(),
                idx_leaf,
            );
            r(
                rsig.as_mut_ptr(),
                rroot.as_mut_ptr(),
                &rc,
                rwa.as_mut_ptr(),
                rta.as_mut_ptr(),
                idx_leaf,
            );
        }
        assert_bytes_eq(&format!("SPX_merkle_sign(idx_leaf={idx_leaf}) sig"), &csig, &rsig);
        assert_bytes_eq(&format!("SPX_merkle_sign(idx_leaf={idx_leaf}) root out"), &croot, &rroot);
        assert_bytes_eq(
            &format!("SPX_merkle_sign(idx_leaf={idx_leaf}) wots_addr after"),
            &addr_bytes(&cwa),
            &addr_bytes(&rwa),
        );
        assert_bytes_eq(
            &format!("SPX_merkle_sign(idx_leaf={idx_leaf}) tree_addr after"),
            &addr_bytes(&cta),
            &addr_bytes(&rta),
        );
    }
}

// --- row 47 -------------------------------------------------------------

#[test]
fn row47_merkle_gen_root() {
    let l = libs();
    let (c, r) = l.pair::<FnMerkleGenRoot>("SPX_merkle_gen_root");
    let mut rng = Rng::new(0x4701);
    for it in 0..16 {
        let (cc, rc) = make_ctx(&mut rng);
        let mut croot = vec![0xA5u8; SPX_N + 8];
        let mut rroot = vec![0xA5u8; SPX_N + 8];
        unsafe {
            c(croot.as_mut_ptr(), &cc);
            r(rroot.as_mut_ptr(), &rc);
        }
        assert_bytes_eq(&format!("SPX_merkle_gen_root #{it}"), &croot, &rroot);
    }
}
