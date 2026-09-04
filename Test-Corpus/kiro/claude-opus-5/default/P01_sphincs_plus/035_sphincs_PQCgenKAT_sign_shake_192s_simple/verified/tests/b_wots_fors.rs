//! Phase B, CONFIGS.md rows 32-40: `app/src/wots.c`, `app/src/fors.c` and
//! `app/src/merkle.c`.

mod common;

use common::params::*;
use common::*;

type ChainLengths = unsafe extern "C" fn(*mut u32, *const u8);
type WotsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *mut u32);
type ForsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const u8, u32, *mut ForsGenLeafInfo);
type ForsSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u32);
type ForsPkFromSig = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, *const u32);
type MerkleSign = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *mut u32, *mut u32, u32);
type MerkleGenRoot = unsafe extern "C" fn(*mut u8, *const u8);

fn messages(rng: &mut Rng, len: usize, n: usize) -> Vec<Vec<u8>> {
    let mut v = vec![vec![0u8; len], vec![0xFFu8; len]];
    // alternating nibbles put the base-w digits at both extremes
    v.push(vec![0x0Fu8; len]);
    v.push(vec![0xF0u8; len]);
    for _ in 0..n {
        v.push(rng.bytes(len));
    }
    v
}

#[test]
fn row32_chain_lengths() {
    let libs = load();
    let (fc, fr) = libs.pair::<ChainLengths>("SPX_chain_lengths");
    let mut rng = Rng::new(32);
    for m in messages(&mut rng, SPX_N, 256) {
        let mut a = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
        let mut b = vec![0xDEAD_BEEFu32; SPX_WOTS_LEN + 4];
        unsafe {
            fc(a.as_mut_ptr(), m.as_ptr());
            fr(b.as_mut_ptr(), m.as_ptr());
        }
        eq(
            &format!("SPX_chain_lengths({})", hex(&m)),
            &u32s_as_bytes(&a),
            &u32s_as_bytes(&b),
        );
        for (i, d) in a[..SPX_WOTS_LEN].iter().enumerate() {
            assert!(
                (*d as usize) < SPX_WOTS_W,
                "digit {i} = {d} is not a base-{SPX_WOTS_W} digit"
            );
        }
    }
}

#[test]
fn row33_wots_pk_from_sig() {
    let libs = load();
    let (fc, fr) = libs.pair::<WotsPkFromSig>("SPX_wots_pk_from_sig");
    let mut rng = Rng::new(33);
    let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    for m in messages(&mut rng, SPX_N, 16) {
        for sigmode in 0..3 {
            let sig = match sigmode {
                0 => vec![0u8; SPX_WOTS_BYTES],
                1 => vec![0xFFu8; SPX_WOTS_BYTES],
                _ => rng.bytes(SPX_WOTS_BYTES),
            };
            let base = rng.addr();
            let mut aa = base;
            let mut ab = base;
            let mut a = vec![0xA5u8; SPX_WOTS_BYTES + 8];
            let mut b = vec![0xA5u8; SPX_WOTS_BYTES + 8];
            unsafe {
                fc(
                    a.as_mut_ptr(),
                    sig.as_ptr(),
                    m.as_ptr(),
                    cc.ptr(),
                    aa.as_mut_ptr(),
                );
                fr(
                    b.as_mut_ptr(),
                    sig.as_ptr(),
                    m.as_ptr(),
                    cr.ptr(),
                    ab.as_mut_ptr(),
                );
            }
            eq("SPX_wots_pk_from_sig pk", &a, &b);
            eq(
                "SPX_wots_pk_from_sig addr",
                &u32s_as_bytes(&aa),
                &u32s_as_bytes(&ab),
            );
        }
    }
}

#[test]
fn row34_fors_gen_leafx1() {
    let libs = load();
    let (fc, fr) = libs.pair::<ForsGenLeafX1>("SPX_fors_gen_leafx1");
    let mut rng = Rng::new(34);
    let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let mut idxs: Vec<u32> = vec![0, 1, 2, 0x7FFF_FFFF, 0xFFFF_FFFE, 0xFFFF_FFFF];
    for _ in 0..64 {
        idxs.push(rng.next_u32());
    }
    for addr_idx in idxs {
        let base = rng.addr();
        let mut ia = ForsGenLeafInfo { leaf_addrx: base };
        let mut ib = ForsGenLeafInfo { leaf_addrx: base };
        let mut a = vec![0xA5u8; SPX_N + 8];
        let mut b = vec![0xA5u8; SPX_N + 8];
        unsafe {
            fc(a.as_mut_ptr(), cc.ptr(), addr_idx, &mut ia);
            fr(b.as_mut_ptr(), cr.ptr(), addr_idx, &mut ib);
        }
        eq(&format!("SPX_fors_gen_leafx1(idx={addr_idx})"), &a, &b);
        eq(
            &format!("SPX_fors_gen_leafx1(idx={addr_idx}) leaf_addrx"),
            &u32s_as_bytes(&ia.leaf_addrx),
            &u32s_as_bytes(&ib.leaf_addrx),
        );
    }
}

fn fors_sign_case(libs: &Libs, rng: &mut Rng, m: &[u8]) -> (Vec<u8>, Vec<u8>, [u32; 8]) {
    let (fc, fr) = libs.pair::<ForsSign>("SPX_fors_sign");
    let (cc, cr) = make_ctx_pair(libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let fors_addr = rng.addr();
    let mut siga = vec![0xA5u8; SPX_FORS_BYTES + 8];
    let mut sigb = vec![0xA5u8; SPX_FORS_BYTES + 8];
    let mut pka = vec![0x5Au8; SPX_N + 8];
    let mut pkb = vec![0x5Au8; SPX_N + 8];
    unsafe {
        fc(
            siga.as_mut_ptr(),
            pka.as_mut_ptr(),
            m.as_ptr(),
            cc.ptr(),
            fors_addr.as_ptr(),
        );
        fr(
            sigb.as_mut_ptr(),
            pkb.as_mut_ptr(),
            m.as_ptr(),
            cr.ptr(),
            fors_addr.as_ptr(),
        );
    }
    eq("SPX_fors_sign sig", &siga, &sigb);
    eq("SPX_fors_sign pk", &pka, &pkb);
    // Return the C context seeds so the caller can re-derive; the ctx pair is
    // identical by construction (make_ctx_pair asserts it).
    let seeds = {
        let mut s = [0u32; 8];
        let b = cc.bytes();
        for (i, w) in s.iter_mut().enumerate() {
            let off = i * 4;
            if off + 4 <= b.len() {
                *w = u32::from_ne_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]]);
            }
        }
        s
    };
    let _ = seeds;
    (siga[..SPX_FORS_BYTES].to_vec(), pka[..SPX_N].to_vec(), fors_addr)
}

#[test]
fn row35_fors_sign_random() {
    let libs = load();
    let mut rng = Rng::new(35);
    for _ in 0..4 {
        let m = rng.bytes(SPX_FORS_MSG_BYTES);
        fors_sign_case(&libs, &mut rng, &m);
    }
}

#[test]
fn row36_fors_sign_index_extremes() {
    let libs = load();
    let mut rng = Rng::new(36);
    for m in [
        vec![0u8; SPX_FORS_MSG_BYTES],
        vec![0xFFu8; SPX_FORS_MSG_BYTES],
        vec![0xAAu8; SPX_FORS_MSG_BYTES],
        vec![0x55u8; SPX_FORS_MSG_BYTES],
    ] {
        fors_sign_case(&libs, &mut rng, &m);
    }
}

#[test]
fn row37_fors_pk_from_sig() {
    let libs = load();
    let (sc, sr) = libs.pair::<ForsSign>("SPX_fors_sign");
    let (vc, vr) = libs.pair::<ForsPkFromSig>("SPX_fors_pk_from_sig");
    let mut rng = Rng::new(37);

    // round trip: fors_pk_from_sig on a genuine signature must reproduce the pk
    for _ in 0..4 {
        let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let m = rng.bytes(SPX_FORS_MSG_BYTES);
        let fors_addr = rng.addr();
        let mut sig = vec![0u8; SPX_FORS_BYTES];
        let mut pk = vec![0u8; SPX_N];
        unsafe {
            sc(
                sig.as_mut_ptr(),
                pk.as_mut_ptr(),
                m.as_ptr(),
                cc.ptr(),
                fors_addr.as_ptr(),
            );
        }
        let mut a = vec![0xA5u8; SPX_N + 8];
        let mut b = vec![0xA5u8; SPX_N + 8];
        unsafe {
            vc(
                a.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                cc.ptr(),
                fors_addr.as_ptr(),
            );
            vr(
                b.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                cr.ptr(),
                fors_addr.as_ptr(),
            );
        }
        eq("SPX_fors_pk_from_sig (genuine)", &a, &b);
        assert_eq!(&a[..SPX_N], &pk[..], "fors round trip pk mismatch");
        let _ = &sr;
    }

    // independent random signatures: the derived pk is garbage but must match
    for _ in 0..8 {
        let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let m = rng.bytes(SPX_FORS_MSG_BYTES);
        let sig = rng.bytes(SPX_FORS_BYTES);
        let fors_addr = rng.addr();
        let mut a = vec![0xA5u8; SPX_N + 8];
        let mut b = vec![0xA5u8; SPX_N + 8];
        unsafe {
            vc(
                a.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                cc.ptr(),
                fors_addr.as_ptr(),
            );
            vr(
                b.as_mut_ptr(),
                sig.as_ptr(),
                m.as_ptr(),
                cr.ptr(),
                fors_addr.as_ptr(),
            );
        }
        eq("SPX_fors_pk_from_sig (random)", &a, &b);
    }
}

fn merkle_sign_case(libs: &Libs, rng: &mut Rng, idx_leaf: u32) {
    let (fc, fr) = libs.pair::<MerkleSign>("SPX_merkle_sign");
    let (cc, cr) = make_ctx_pair(libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
    let siglen = SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
    let root_in = rng.bytes(SPX_N);
    let wots_addr = rng.addr();
    let tree_addr = rng.addr();

    let mut siga = vec![0xA5u8; siglen + 8];
    let mut sigb = vec![0xA5u8; siglen + 8];
    let mut roota = root_in.clone();
    let mut rootb = root_in.clone();
    let mut wa = wots_addr;
    let mut wb = wots_addr;
    let mut ta = tree_addr;
    let mut tb = tree_addr;
    unsafe {
        fc(
            siga.as_mut_ptr(),
            roota.as_mut_ptr(),
            cc.ptr(),
            wa.as_mut_ptr(),
            ta.as_mut_ptr(),
            idx_leaf,
        );
        fr(
            sigb.as_mut_ptr(),
            rootb.as_mut_ptr(),
            cr.ptr(),
            wb.as_mut_ptr(),
            tb.as_mut_ptr(),
            idx_leaf,
        );
    }
    let what = format!("SPX_merkle_sign(idx_leaf={idx_leaf})");
    eq(&format!("{what} sig"), &siga, &sigb);
    eq(&format!("{what} root"), &roota, &rootb);
    eq(&format!("{what} wots_addr"), &u32s_as_bytes(&wa), &u32s_as_bytes(&wb));
    eq(&format!("{what} tree_addr"), &u32s_as_bytes(&ta), &u32s_as_bytes(&tb));
}

#[test]
fn row38_merkle_sign_random() {
    let libs = load();
    let mut rng = Rng::new(38);
    let n = 1u32 << SPX_TREE_HEIGHT;
    for _ in 0..4 {
        let idx = rng.below(n);
        merkle_sign_case(&libs, &mut rng, idx);
    }
}

#[test]
fn row39_merkle_sign_extremes() {
    let libs = load();
    let mut rng = Rng::new(39);
    let n = 1u32 << SPX_TREE_HEIGHT;
    for idx in [0u32, 1, n - 1, u32::MAX] {
        merkle_sign_case(&libs, &mut rng, idx);
    }
}

#[test]
fn row40_merkle_gen_root() {
    let libs = load();
    let (fc, fr) = libs.pair::<MerkleGenRoot>("SPX_merkle_gen_root");
    let mut rng = Rng::new(40);
    for _ in 0..2 {
        let (cc, cr) = make_ctx_pair(&libs, &rng.bytes(SPX_N), &rng.bytes(SPX_N));
        let mut a = vec![0xA5u8; SPX_N + 8];
        let mut b = vec![0xA5u8; SPX_N + 8];
        unsafe {
            fc(a.as_mut_ptr(), cc.ptr());
            fr(b.as_mut_ptr(), cr.ptr());
        }
        eq("SPX_merkle_gen_root", &a, &b);
    }
    for seed in [vec![0xFFu8; SPX_N]] {
        let (cc, cr) = make_ctx_pair(&libs, &seed, &seed);
        let mut a = vec![0u8; SPX_N];
        let mut b = vec![0u8; SPX_N];
        unsafe {
            fc(a.as_mut_ptr(), cc.ptr());
            fr(b.as_mut_ptr(), cr.ptr());
        }
        eq("SPX_merkle_gen_root (extreme seed)", &a, &b);
    }
}
