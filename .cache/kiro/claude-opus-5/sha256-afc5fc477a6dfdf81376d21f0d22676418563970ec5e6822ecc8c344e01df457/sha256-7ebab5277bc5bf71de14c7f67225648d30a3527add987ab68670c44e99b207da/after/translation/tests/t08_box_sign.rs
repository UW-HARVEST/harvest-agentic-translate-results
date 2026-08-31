//! Public-key layer: crypto_box (easy / detached / beforenm-afternm / seal /
//! NaCl API, plus the curve25519xchacha20poly1305 variant) and crypto_sign
//! (ed25519, detached, multi-part ed25519ph, key conversions).
mod common;

use common::*;
use std::os::raw::{c_int, c_uchar, c_ulonglong, c_void};

type FnSeedKeypair = unsafe extern "C" fn(*mut c_uchar, *mut c_uchar, *const c_uchar) -> c_int;
type FnKeypair = unsafe extern "C" fn(*mut c_uchar, *mut c_uchar) -> c_int;
type FnBox = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
    *const c_uchar,
) -> c_int;
type FnBoxDetached = unsafe extern "C" fn(
    *mut c_uchar,
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
    *const c_uchar,
) -> c_int;
type FnBoxOpenDetached = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
    *const c_uchar,
) -> c_int;
type FnBeforenm = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, *const c_uchar) -> c_int;
type FnAfternm = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
) -> c_int;
type FnDetachedAfternm = unsafe extern "C" fn(
    *mut c_uchar,
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
) -> c_int;
type FnOpenDetachedAfternm = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
) -> c_int;
type FnSeal = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, c_ulonglong, *const c_uchar) -> c_int;
type FnSealOpen = unsafe extern "C" fn(
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    *const c_uchar,
) -> c_int;

struct BoxDims {
    pkb: usize,
    skb: usize,
    nb: usize,
    mb: usize,
    sdb: usize,
    bnb: usize,
}

fn box_dims(prefix: &str) -> BoxDims {
    unsafe {
        let g = |s: &str| -> usize {
            let (c, _): (FnSize, FnSize) = pair(&format!("{prefix}_{s}"));
            c()
        };
        BoxDims {
            pkb: g("publickeybytes"),
            skb: g("secretkeybytes"),
            nb: g("noncebytes"),
            mb: g("macbytes"),
            sdb: g("seedbytes"),
            bnb: g("beforenmbytes"),
        }
    }
}

fn box_suite(prefix: &str) {
    for s in [
        "seedbytes",
        "publickeybytes",
        "secretkeybytes",
        "noncebytes",
        "macbytes",
        "messagebytes_max",
        "beforenmbytes",
    ] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    let d = box_dims(prefix);
    unsafe {
        let (csk, rsk): (FnSeedKeypair, FnSeedKeypair) = pair(&format!("{prefix}_seed_keypair"));
        let (ckp, rkp): (FnKeypair, FnKeypair) = pair(&format!("{prefix}_keypair"));

        let mut rng = Rng::new(0x6000 + prefix.len() as u64);

        // seed_keypair over structured and random seeds
        let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; d.sdb], vec![0xffu8; d.sdb]];
        for _ in 0..5 {
            seeds.push(rng.vec(d.sdb));
        }
        let mut kps: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for seed in &seeds {
            let mut cpk = vec![0xAAu8; d.pkb + 8];
            let mut rpk = vec![0xAAu8; d.pkb + 8];
            let mut csk_b = vec![0xAAu8; d.skb + 8];
            let mut rsk_b = vec![0xAAu8; d.skb + 8];
            let a = csk(cpk.as_mut_ptr(), csk_b.as_mut_ptr(), seed.as_ptr());
            let b = rsk(rpk.as_mut_ptr(), rsk_b.as_mut_ptr(), seed.as_ptr());
            assert_eq!(a, b, "{prefix}_seed_keypair return");
            assert_bytes_eq(&format!("{prefix}_seed_keypair pk"), &cpk, &rpk);
            assert_bytes_eq(&format!("{prefix}_seed_keypair sk"), &csk_b, &rsk_b);
            kps.push((cpk[..d.pkb].to_vec(), csk_b[..d.skb].to_vec()));
        }
        // keypair via the shared deterministic RNG
        for _ in 0..5 {
            let mut cpk = vec![0xAAu8; d.pkb + 8];
            let mut rpk = vec![0xAAu8; d.pkb + 8];
            let mut csk_b = vec![0xAAu8; d.skb + 8];
            let mut rsk_b = vec![0xAAu8; d.skb + 8];
            det_reset();
            let a = ckp(cpk.as_mut_ptr(), csk_b.as_mut_ptr());
            det_reset();
            let b = rkp(rpk.as_mut_ptr(), rsk_b.as_mut_ptr());
            assert_eq!(a, b, "{prefix}_keypair return");
            assert_bytes_eq(&format!("{prefix}_keypair pk"), &cpk, &rpk);
            assert_bytes_eq(&format!("{prefix}_keypair sk"), &csk_b, &rsk_b);
        }

        let mut nonces: Vec<Vec<u8>> = vec![vec![0u8; d.nb], vec![0xffu8; d.nb]];
        nonces.push(rng.vec(d.nb));
        let msg = rng.vec(3001);
        let mlens: Vec<usize> = vec![
            0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 255, 256, 1000, 3000,
        ];

        // degenerate peer keys must be rejected identically
        let mut peer_pks: Vec<Vec<u8>> = kps.iter().map(|(p, _)| p.clone()).collect();
        peer_pks.push(vec![0u8; d.pkb]);
        peer_pks.push(vec![0xffu8; d.pkb]);
        peer_pks.push(rng.vec(d.pkb));

        let (_mypk, mysk) = (&kps[2].0, &kps[2].1);
        if has(&format!("{prefix}_easy")) {
        let (ce, re): (FnBox, FnBox) = pair(&format!("{prefix}_easy"));
        let (co_, ro_): (FnBox, FnBox) = pair(&format!("{prefix}_open_easy"));
        let (cd, rd): (FnBoxDetached, FnBoxDetached) = pair(&format!("{prefix}_detached"));
        let (cod, rod): (FnBoxOpenDetached, FnBoxOpenDetached) =
            pair(&format!("{prefix}_open_detached"));

        for peer in &peer_pks {
            for nonce in nonces.iter().take(2) {
                for &mlen in &mlens {
                    let mut cc = vec![0xAAu8; mlen + d.mb + 8];
                    let mut rc = vec![0xAAu8; mlen + d.mb + 8];
                    let a = ce(
                        cc.as_mut_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        peer.as_ptr(),
                        mysk.as_ptr(),
                    );
                    let b = re(
                        rc.as_mut_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        peer.as_ptr(),
                        mysk.as_ptr(),
                    );
                    let tag = format!("{prefix}_easy(mlen={mlen},pk={})", hex(peer));
                    assert_eq!(a, b, "{tag} return");
                    assert_bytes_eq(&tag, &cc, &rc);

                    let clen = mlen + d.mb;
                    let mut cm = vec![0xAAu8; clen + 8];
                    let mut rm = vec![0xAAu8; clen + 8];
                    let a = co_(
                        cm.as_mut_ptr(),
                        cc.as_ptr(),
                        clen as c_ulonglong,
                        nonce.as_ptr(),
                        peer.as_ptr(),
                        mysk.as_ptr(),
                    );
                    let b = ro_(
                        rm.as_mut_ptr(),
                        cc.as_ptr(),
                        clen as c_ulonglong,
                        nonce.as_ptr(),
                        peer.as_ptr(),
                        mysk.as_ptr(),
                    );
                    let otag = format!("{prefix}_open_easy(mlen={mlen})");
                    assert_eq!(a, b, "{otag} return");
                    assert_bytes_eq(&otag, &cm, &rm);

                    // tampered / truncated
                    let mut bads: Vec<Vec<u8>> = vec![Vec::new()];
                    if clen > 0 {
                        let mut v = cc[..clen].to_vec();
                        v[0] ^= 1;
                        bads.push(v);
                        bads.push(cc[..clen - 1].to_vec());
                    }
                    bads.push(cc[..d.mb.saturating_sub(1)].to_vec());
                    for bad in bads {
                        let mut cm = vec![0xAAu8; clen + 8];
                        let mut rm = vec![0xAAu8; clen + 8];
                        let a = co_(
                            cm.as_mut_ptr(),
                            bad.as_ptr(),
                            bad.len() as c_ulonglong,
                            nonce.as_ptr(),
                            peer.as_ptr(),
                            mysk.as_ptr(),
                        );
                        let b = ro_(
                            rm.as_mut_ptr(),
                            bad.as_ptr(),
                            bad.len() as c_ulonglong,
                            nonce.as_ptr(),
                            peer.as_ptr(),
                            mysk.as_ptr(),
                        );
                        assert_eq!(a, b, "{otag} bad(len={}) return", bad.len());
                        assert_bytes_eq(&format!("{otag} bad(len={})", bad.len()), &cm, &rm);
                    }

                    // detached
                    let mut cc = vec![0xAAu8; mlen + 8];
                    let mut rc = vec![0xAAu8; mlen + 8];
                    let mut cmac = vec![0xAAu8; d.mb + 8];
                    let mut rmac = vec![0xAAu8; d.mb + 8];
                    let a = cd(
                        cc.as_mut_ptr(),
                        cmac.as_mut_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        peer.as_ptr(),
                        mysk.as_ptr(),
                    );
                    let b = rd(
                        rc.as_mut_ptr(),
                        rmac.as_mut_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        peer.as_ptr(),
                        mysk.as_ptr(),
                    );
                    let dtag = format!("{prefix}_detached(mlen={mlen})");
                    assert_eq!(a, b, "{dtag} return");
                    assert_bytes_eq(&format!("{dtag} c"), &cc, &rc);
                    assert_bytes_eq(&format!("{dtag} mac"), &cmac, &rmac);

                    let mut cm = vec![0xAAu8; mlen + 8];
                    let mut rm = vec![0xAAu8; mlen + 8];
                    let a = cod(
                        cm.as_mut_ptr(),
                        cc.as_ptr(),
                        cmac.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        peer.as_ptr(),
                        mysk.as_ptr(),
                    );
                    let b = rod(
                        rm.as_mut_ptr(),
                        cc.as_ptr(),
                        cmac.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        peer.as_ptr(),
                        mysk.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_open_detached return mlen={mlen}");
                    assert_bytes_eq(&format!("{prefix}_open_detached mlen={mlen}"), &cm, &rm);

                    let mut badmac = cmac[..d.mb].to_vec();
                    badmac[0] ^= 1;
                    let mut cm = vec![0xAAu8; mlen + 8];
                    let mut rm = vec![0xAAu8; mlen + 8];
                    let a = cod(
                        cm.as_mut_ptr(),
                        cc.as_ptr(),
                        badmac.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        peer.as_ptr(),
                        mysk.as_ptr(),
                    );
                    let b = rod(
                        rm.as_mut_ptr(),
                        cc.as_ptr(),
                        badmac.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        peer.as_ptr(),
                        mysk.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_open_detached bad mac return");
                    assert_bytes_eq(&format!("{prefix}_open_detached bad mac"), &cm, &rm);
                }
            }
        }

        }

        // beforenm / afternm
        let (cbn, rbn): (FnBeforenm, FnBeforenm) = pair(&format!("{prefix}_beforenm"));
        let have_easy_afternm = has(&format!("{prefix}_easy_afternm"));

        for peer in &peer_pks {
            let mut ck_b = vec![0xAAu8; d.bnb + 8];
            let mut rk_b = vec![0xAAu8; d.bnb + 8];
            let a = cbn(ck_b.as_mut_ptr(), peer.as_ptr(), mysk.as_ptr());
            let b = rbn(rk_b.as_mut_ptr(), peer.as_ptr(), mysk.as_ptr());
            assert_eq!(a, b, "{prefix}_beforenm return");
            assert_bytes_eq(&format!("{prefix}_beforenm"), &ck_b, &rk_b);
            if a != 0 {
                continue;
            }
            let k = ck_b[..d.bnb].to_vec();
            if !have_easy_afternm {
                continue;
            }
            let (cea, rea): (FnAfternm, FnAfternm) = pair(&format!("{prefix}_easy_afternm"));
            let (coa, roa): (FnAfternm, FnAfternm) = pair(&format!("{prefix}_open_easy_afternm"));
            let (cda, rda): (FnDetachedAfternm, FnDetachedAfternm) =
                pair(&format!("{prefix}_detached_afternm"));
            let (coda, roda): (FnOpenDetachedAfternm, FnOpenDetachedAfternm) =
                pair(&format!("{prefix}_open_detached_afternm"));
            for nonce in nonces.iter().take(2) {
                for &mlen in &[0usize, 1, 32, 33, 64, 128, 1000] {
                    let mut cc = vec![0xAAu8; mlen + d.mb + 8];
                    let mut rc = vec![0xAAu8; mlen + d.mb + 8];
                    let a = cea(
                        cc.as_mut_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        k.as_ptr(),
                    );
                    let b = rea(
                        rc.as_mut_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        k.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_easy_afternm return mlen={mlen}");
                    assert_bytes_eq(&format!("{prefix}_easy_afternm mlen={mlen}"), &cc, &rc);

                    let clen = mlen + d.mb;
                    let mut cm = vec![0xAAu8; clen + 8];
                    let mut rm = vec![0xAAu8; clen + 8];
                    let a = coa(
                        cm.as_mut_ptr(),
                        cc.as_ptr(),
                        clen as c_ulonglong,
                        nonce.as_ptr(),
                        k.as_ptr(),
                    );
                    let b = roa(
                        rm.as_mut_ptr(),
                        cc.as_ptr(),
                        clen as c_ulonglong,
                        nonce.as_ptr(),
                        k.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_open_easy_afternm return mlen={mlen}");
                    assert_bytes_eq(&format!("{prefix}_open_easy_afternm mlen={mlen}"), &cm, &rm);

                    // short ciphertext
                    let mut cm = vec![0xAAu8; clen + 8];
                    let mut rm = vec![0xAAu8; clen + 8];
                    let short = d.mb.saturating_sub(1);
                    let a = coa(
                        cm.as_mut_ptr(),
                        cc.as_ptr(),
                        short as c_ulonglong,
                        nonce.as_ptr(),
                        k.as_ptr(),
                    );
                    let b = roa(
                        rm.as_mut_ptr(),
                        cc.as_ptr(),
                        short as c_ulonglong,
                        nonce.as_ptr(),
                        k.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_open_easy_afternm short return");
                    assert_bytes_eq(&format!("{prefix}_open_easy_afternm short"), &cm, &rm);

                    let mut cc = vec![0xAAu8; mlen + 8];
                    let mut rc = vec![0xAAu8; mlen + 8];
                    let mut cmac = vec![0xAAu8; d.mb + 8];
                    let mut rmac = vec![0xAAu8; d.mb + 8];
                    let a = cda(
                        cc.as_mut_ptr(),
                        cmac.as_mut_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        k.as_ptr(),
                    );
                    let b = rda(
                        rc.as_mut_ptr(),
                        rmac.as_mut_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        k.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_detached_afternm return");
                    assert_bytes_eq(&format!("{prefix}_detached_afternm c"), &cc, &rc);
                    assert_bytes_eq(&format!("{prefix}_detached_afternm mac"), &cmac, &rmac);

                    let mut cm = vec![0xAAu8; mlen + 8];
                    let mut rm = vec![0xAAu8; mlen + 8];
                    let a = coda(
                        cm.as_mut_ptr(),
                        cc.as_ptr(),
                        cmac.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        k.as_ptr(),
                    );
                    let b = roda(
                        rm.as_mut_ptr(),
                        cc.as_ptr(),
                        cmac.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        k.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_open_detached_afternm return");
                    assert_bytes_eq(&format!("{prefix}_open_detached_afternm"), &cm, &rm);
                }
            }
        }

        // seal / seal_open (consume randomness -> reset the RNG each call)
        if has(&format!("{prefix}_seal")) {
        cmp_size(&format!("{prefix}_sealbytes"));
        let (csb, _): (FnSize, FnSize) = pair(&format!("{prefix}_sealbytes"));
        let sealb = csb();
        let (cs, rs): (FnSeal, FnSeal) = pair(&format!("{prefix}_seal"));
        let (cso, rso): (FnSealOpen, FnSealOpen) = pair(&format!("{prefix}_seal_open"));
        for (pk, sk) in kps.iter().take(4) {
            for &mlen in &[0usize, 1, 32, 33, 64, 128, 1000, 3000] {
                let mut cc = vec![0xAAu8; mlen + sealb + 8];
                let mut rc = vec![0xAAu8; mlen + sealb + 8];
                det_reset();
                let a = cs(
                    cc.as_mut_ptr(),
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    pk.as_ptr(),
                );
                det_reset();
                let b = rs(
                    rc.as_mut_ptr(),
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    pk.as_ptr(),
                );
                let tag = format!("{prefix}_seal(mlen={mlen})");
                assert_eq!(a, b, "{tag} return");
                assert_bytes_eq(&tag, &cc, &rc);

                let clen = mlen + sealb;
                let mut cm = vec![0xAAu8; clen + 8];
                let mut rm = vec![0xAAu8; clen + 8];
                let a = cso(
                    cm.as_mut_ptr(),
                    cc.as_ptr(),
                    clen as c_ulonglong,
                    pk.as_ptr(),
                    sk.as_ptr(),
                );
                let b = rso(
                    rm.as_mut_ptr(),
                    cc.as_ptr(),
                    clen as c_ulonglong,
                    pk.as_ptr(),
                    sk.as_ptr(),
                );
                let otag = format!("{prefix}_seal_open(mlen={mlen})");
                assert_eq!(a, b, "{otag} return");
                assert_bytes_eq(&otag, &cm, &rm);
                assert_eq!(a, 0, "{otag} should succeed");

                let mut bads: Vec<Vec<u8>> = vec![Vec::new(), cc[..sealb - 1].to_vec()];
                let mut v = cc[..clen].to_vec();
                v[0] ^= 1;
                bads.push(v);
                let mut v = cc[..clen].to_vec();
                v[clen - 1] ^= 0x80;
                bads.push(v);
                for bad in bads {
                    let mut cm = vec![0xAAu8; clen + 8];
                    let mut rm = vec![0xAAu8; clen + 8];
                    let a = cso(
                        cm.as_mut_ptr(),
                        bad.as_ptr(),
                        bad.len() as c_ulonglong,
                        pk.as_ptr(),
                        sk.as_ptr(),
                    );
                    let b = rso(
                        rm.as_mut_ptr(),
                        bad.as_ptr(),
                        bad.len() as c_ulonglong,
                        pk.as_ptr(),
                        sk.as_ptr(),
                    );
                    assert_eq!(a, b, "{otag} bad(len={}) return", bad.len());
                    assert_bytes_eq(&format!("{otag} bad(len={})", bad.len()), &cm, &rm);
                }
            }
        }

        }

        // deprecated NaCl API
        if has(&format!("{prefix}_zerobytes")) {
            cmp_size(&format!("{prefix}_zerobytes"));
            cmp_size(&format!("{prefix}_boxzerobytes"));
            let (czb, _): (FnSize, FnSize) = pair(&format!("{prefix}_zerobytes"));
            let (cbzb, _): (FnSize, FnSize) = pair(&format!("{prefix}_boxzerobytes"));
            let zb = czb();
            let bzb = cbzb();
            let (cn, rn): (FnBox, FnBox) = pair(prefix);
            let (cn_o, rn_o): (FnBox, FnBox) = pair(&format!("{prefix}_open"));
            let (cna, rna): (FnAfternm, FnAfternm) = pair(&format!("{prefix}_afternm"));
            let (cnoa, rnoa): (FnAfternm, FnAfternm) = pair(&format!("{prefix}_open_afternm"));
            let (pk, sk) = &kps[1];
            let mut kbuf = vec![0u8; d.bnb];
            assert_eq!(cbn(kbuf.as_mut_ptr(), pk.as_ptr(), sk.as_ptr()), 0);
            for nonce in nonces.iter().take(2) {
                for &plen in &[0usize, 1, 32, 33, 128, 1000] {
                    let mlen = zb + plen;
                    let mut m = vec![0u8; mlen];
                    rng.fill(&mut m[zb..]);
                    let mut cc = vec![0xAAu8; mlen + 8];
                    let mut rc = vec![0xAAu8; mlen + 8];
                    let a = cn(
                        cc.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        pk.as_ptr(),
                        sk.as_ptr(),
                    );
                    let b = rn(
                        rc.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        pk.as_ptr(),
                        sk.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix} nacl return mlen={mlen}");
                    assert_bytes_eq(&format!("{prefix} nacl mlen={mlen}"), &cc, &rc);

                    let mut cm = vec![0xAAu8; mlen + 8];
                    let mut rm = vec![0xAAu8; mlen + 8];
                    let a = cn_o(
                        cm.as_mut_ptr(),
                        cc.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        pk.as_ptr(),
                        sk.as_ptr(),
                    );
                    let b = rn_o(
                        rm.as_mut_ptr(),
                        cc.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        pk.as_ptr(),
                        sk.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_open nacl return mlen={mlen}");
                    assert_bytes_eq(&format!("{prefix}_open nacl mlen={mlen}"), &cm, &rm);

                    // too-short input
                    let short = zb.saturating_sub(1).min(mlen);
                    let mut cm = vec![0xAAu8; mlen + 8];
                    let mut rm = vec![0xAAu8; mlen + 8];
                    let a = cn_o(
                        cm.as_mut_ptr(),
                        cc.as_ptr(),
                        short as c_ulonglong,
                        nonce.as_ptr(),
                        pk.as_ptr(),
                        sk.as_ptr(),
                    );
                    let b = rn_o(
                        rm.as_mut_ptr(),
                        cc.as_ptr(),
                        short as c_ulonglong,
                        nonce.as_ptr(),
                        pk.as_ptr(),
                        sk.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_open nacl short return");
                    assert_bytes_eq(&format!("{prefix}_open nacl short"), &cm, &rm);

                    // afternm variants
                    let mut cc = vec![0xAAu8; mlen + 8];
                    let mut rc = vec![0xAAu8; mlen + 8];
                    let a = cna(
                        cc.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        kbuf.as_ptr(),
                    );
                    let b = rna(
                        rc.as_mut_ptr(),
                        m.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        kbuf.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_afternm return");
                    assert_bytes_eq(&format!("{prefix}_afternm mlen={mlen}"), &cc, &rc);

                    let mut cm = vec![0xAAu8; mlen + 8];
                    let mut rm = vec![0xAAu8; mlen + 8];
                    let a = cnoa(
                        cm.as_mut_ptr(),
                        cc.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        kbuf.as_ptr(),
                    );
                    let b = rnoa(
                        rm.as_mut_ptr(),
                        cc.as_ptr(),
                        mlen as c_ulonglong,
                        nonce.as_ptr(),
                        kbuf.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_open_afternm return");
                    assert_bytes_eq(&format!("{prefix}_open_afternm mlen={mlen}"), &cm, &rm);
                    let _ = bzb;
                }
            }
        }
    }
}

#[test]
fn crypto_box_matches() {
    cmp_cstr("crypto_box_primitive");
    box_suite("crypto_box");
}

#[test]
fn crypto_box_curve25519xsalsa20poly1305_matches() {
    box_suite("crypto_box_curve25519xsalsa20poly1305");
}

#[test]
fn crypto_box_curve25519xchacha20poly1305_matches() {
    box_suite("crypto_box_curve25519xchacha20poly1305");
}

// ---------------------------------------------------------------------------
// crypto_sign
// ---------------------------------------------------------------------------

type FnSign = unsafe extern "C" fn(
    *mut c_uchar,
    *mut c_ulonglong,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
) -> c_int;
type FnSignOpen = unsafe extern "C" fn(
    *mut c_uchar,
    *mut c_ulonglong,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
) -> c_int;
type FnVerifyDetached =
    unsafe extern "C" fn(*const c_uchar, *const c_uchar, c_ulonglong, *const c_uchar) -> c_int;
type FnConvert = unsafe extern "C" fn(*mut c_uchar, *const c_uchar) -> c_int;
type FnPhInit = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnPhUpdate = unsafe extern "C" fn(*mut c_void, *const c_uchar, c_ulonglong) -> c_int;
type FnPhFinalCreate =
    unsafe extern "C" fn(*mut c_void, *mut c_uchar, *mut c_ulonglong, *const c_uchar) -> c_int;
type FnPhFinalVerify = unsafe extern "C" fn(*mut c_void, *const c_uchar, *const c_uchar) -> c_int;

fn sign_suite(prefix: &str) {
    for s in [
        "bytes",
        "seedbytes",
        "publickeybytes",
        "secretkeybytes",
        "messagebytes_max",
    ] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    unsafe {
        let g = |s: &str| -> usize {
            let (c, _): (FnSize, FnSize) = pair(&format!("{prefix}_{s}"));
            c()
        };
        let sigb = g("bytes");
        let sdb = g("seedbytes");
        let pkb = g("publickeybytes");
        let skb = g("secretkeybytes");

        let (csk, rsk): (FnSeedKeypair, FnSeedKeypair) = pair(&format!("{prefix}_seed_keypair"));
        let (ckp, rkp): (FnKeypair, FnKeypair) = pair(&format!("{prefix}_keypair"));
        let (cs, rs): (FnSign, FnSign) = pair(prefix);
        let (cso, rso): (FnSignOpen, FnSignOpen) = pair(&format!("{prefix}_open"));
        let (cdt, rdt): (FnSign, FnSign) = pair(&format!("{prefix}_detached"));
        let (cvd, rvd): (FnVerifyDetached, FnVerifyDetached) =
            pair(&format!("{prefix}_verify_detached"));

        let mut rng = Rng::new(0x6100 + prefix.len() as u64);
        let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; sdb], vec![0xffu8; sdb]];
        for _ in 0..4 {
            seeds.push(rng.vec(sdb));
        }
        let mut kps: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for seed in &seeds {
            let mut cpk = vec![0xAAu8; pkb + 8];
            let mut rpk = vec![0xAAu8; pkb + 8];
            let mut csk_b = vec![0xAAu8; skb + 8];
            let mut rsk_b = vec![0xAAu8; skb + 8];
            let a = csk(cpk.as_mut_ptr(), csk_b.as_mut_ptr(), seed.as_ptr());
            let b = rsk(rpk.as_mut_ptr(), rsk_b.as_mut_ptr(), seed.as_ptr());
            assert_eq!(a, b, "{prefix}_seed_keypair return");
            assert_bytes_eq(&format!("{prefix}_seed_keypair pk"), &cpk, &rpk);
            assert_bytes_eq(&format!("{prefix}_seed_keypair sk"), &csk_b, &rsk_b);
            kps.push((cpk[..pkb].to_vec(), csk_b[..skb].to_vec()));
        }
        for _ in 0..4 {
            let mut cpk = vec![0xAAu8; pkb + 8];
            let mut rpk = vec![0xAAu8; pkb + 8];
            let mut csk_b = vec![0xAAu8; skb + 8];
            let mut rsk_b = vec![0xAAu8; skb + 8];
            det_reset();
            let a = ckp(cpk.as_mut_ptr(), csk_b.as_mut_ptr());
            det_reset();
            let b = rkp(rpk.as_mut_ptr(), rsk_b.as_mut_ptr());
            assert_eq!(a, b, "{prefix}_keypair return");
            assert_bytes_eq(&format!("{prefix}_keypair pk"), &cpk, &rpk);
            assert_bytes_eq(&format!("{prefix}_keypair sk"), &csk_b, &rsk_b);
        }

        let msg = rng.vec(3001);
        let mlens: Vec<usize> = vec![
            0, 1, 2, 15, 16, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 255, 256, 1000, 3000,
        ];

        for (pk, sk) in kps.iter() {
            for &mlen in &mlens {
                // combined sign
                let mut csm = vec![0xAAu8; mlen + sigb + 8];
                let mut rsm = vec![0xAAu8; mlen + sigb + 8];
                let mut cl: c_ulonglong = 0xdead;
                let mut rl: c_ulonglong = 0xdead;
                let a = cs(
                    csm.as_mut_ptr(),
                    &mut cl,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    sk.as_ptr(),
                );
                let b = rs(
                    rsm.as_mut_ptr(),
                    &mut rl,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    sk.as_ptr(),
                );
                let tag = format!("{prefix}(mlen={mlen})");
                assert_eq!(a, b, "{tag} return");
                assert_eq!(cl, rl, "{tag} smlen");
                assert_bytes_eq(&tag, &csm, &rsm);

                // NULL smlen_p
                let mut csm2 = vec![0xAAu8; mlen + sigb + 8];
                let mut rsm2 = vec![0xAAu8; mlen + sigb + 8];
                cs(
                    csm2.as_mut_ptr(),
                    std::ptr::null_mut(),
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    sk.as_ptr(),
                );
                rs(
                    rsm2.as_mut_ptr(),
                    std::ptr::null_mut(),
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    sk.as_ptr(),
                );
                assert_bytes_eq(&format!("{tag} NULL smlen_p"), &csm2, &rsm2);

                // open
                let smlen = cl as usize;
                let mut cm = vec![0xAAu8; smlen + 8];
                let mut rm = vec![0xAAu8; smlen + 8];
                let mut cl2: c_ulonglong = 0xdead;
                let mut rl2: c_ulonglong = 0xdead;
                let a = cso(
                    cm.as_mut_ptr(),
                    &mut cl2,
                    csm.as_ptr(),
                    smlen as c_ulonglong,
                    pk.as_ptr(),
                );
                let b = rso(
                    rm.as_mut_ptr(),
                    &mut rl2,
                    csm.as_ptr(),
                    smlen as c_ulonglong,
                    pk.as_ptr(),
                );
                let otag = format!("{prefix}_open(mlen={mlen})");
                assert_eq!(a, b, "{otag} return");
                assert_eq!(cl2, rl2, "{otag} mlen");
                assert_bytes_eq(&otag, &cm, &rm);
                assert_eq!(a, 0, "{otag} should succeed");

                // tampered / truncated signed messages
                let mut bads: Vec<Vec<u8>> = vec![Vec::new(), csm[..sigb - 1].to_vec()];
                let mut v = csm[..smlen].to_vec();
                v[0] ^= 1;
                bads.push(v);
                let mut v = csm[..smlen].to_vec();
                v[sigb - 1] ^= 0x80;
                bads.push(v);
                if mlen > 0 {
                    let mut v = csm[..smlen].to_vec();
                    v[sigb] ^= 1;
                    bads.push(v);
                }
                bads.push(csm[..smlen - 1].to_vec());
                for bad in bads {
                    let mut cm = vec![0xAAu8; smlen + 8];
                    let mut rm = vec![0xAAu8; smlen + 8];
                    let mut cl2: c_ulonglong = 0xdead;
                    let mut rl2: c_ulonglong = 0xdead;
                    let a = cso(
                        cm.as_mut_ptr(),
                        &mut cl2,
                        bad.as_ptr(),
                        bad.len() as c_ulonglong,
                        pk.as_ptr(),
                    );
                    let b = rso(
                        rm.as_mut_ptr(),
                        &mut rl2,
                        bad.as_ptr(),
                        bad.len() as c_ulonglong,
                        pk.as_ptr(),
                    );
                    assert_eq!(a, b, "{otag} bad(len={}) return", bad.len());
                    assert_eq!(cl2, rl2, "{otag} bad(len={}) mlen", bad.len());
                    assert_bytes_eq(&format!("{otag} bad(len={})", bad.len()), &cm, &rm);
                }

                // detached
                let mut csig = vec![0xAAu8; sigb + 8];
                let mut rsig = vec![0xAAu8; sigb + 8];
                let mut cl3: c_ulonglong = 0xdead;
                let mut rl3: c_ulonglong = 0xdead;
                let a = cdt(
                    csig.as_mut_ptr(),
                    &mut cl3,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    sk.as_ptr(),
                );
                let b = rdt(
                    rsig.as_mut_ptr(),
                    &mut rl3,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    sk.as_ptr(),
                );
                let dtag = format!("{prefix}_detached(mlen={mlen})");
                assert_eq!(a, b, "{dtag} return");
                assert_eq!(cl3, rl3, "{dtag} siglen");
                assert_bytes_eq(&dtag, &csig, &rsig);

                let a = cvd(csig.as_ptr(), msg.as_ptr(), mlen as c_ulonglong, pk.as_ptr());
                let b = rvd(csig.as_ptr(), msg.as_ptr(), mlen as c_ulonglong, pk.as_ptr());
                assert_eq!(a, b, "{prefix}_verify_detached good return");
                assert_eq!(a, 0, "{prefix}_verify_detached should succeed");

                // bad signatures: bit flips, all-zero, all-ones, non-canonical S
                let mut badsigs: Vec<Vec<u8>> =
                    vec![vec![0u8; sigb], vec![0xffu8; sigb]];
                for bit in [0usize, 7, 255, 256, sigb * 8 - 1] {
                    let mut v = csig[..sigb].to_vec();
                    v[bit / 8] ^= 1 << (bit % 8);
                    badsigs.push(v);
                }
                // S = L (non-canonical scalar)
                let l: [u8; 32] = [
                    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde,
                    0xf9, 0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
                ];
                if sigb == 64 {
                    let mut v = csig[..sigb].to_vec();
                    v[32..].copy_from_slice(&l);
                    badsigs.push(v);
                    let mut v = csig[..sigb].to_vec();
                    v[63] |= 0xe0;
                    badsigs.push(v);
                }
                for bad in &badsigs {
                    let a = cvd(bad.as_ptr(), msg.as_ptr(), mlen as c_ulonglong, pk.as_ptr());
                    let b = rvd(bad.as_ptr(), msg.as_ptr(), mlen as c_ulonglong, pk.as_ptr());
                    assert_eq!(a, b, "{prefix}_verify_detached bad({}) return", hex(bad));
                }
                // bad public keys
                let mut badpks: Vec<Vec<u8>> = vec![vec![0u8; pkb], vec![0xffu8; pkb]];
                let mut v = pk.clone();
                v[0] ^= 1;
                badpks.push(v);
                let mut v = pk.clone();
                v[pkb - 1] ^= 0x80;
                badpks.push(v);
                for badpk in &badpks {
                    let a = cvd(
                        csig.as_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        badpk.as_ptr(),
                    );
                    let b = rvd(
                        csig.as_ptr(),
                        msg.as_ptr(),
                        mlen as c_ulonglong,
                        badpk.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_verify_detached badpk({}) return", hex(badpk));
                }
            }
        }
    }
}

#[test]
fn crypto_sign_ed25519_matches() {
    sign_suite("crypto_sign_ed25519");
}

#[test]
fn crypto_sign_generic_matches() {
    cmp_cstr("crypto_sign_primitive");
    cmp_size("crypto_sign_statebytes");
    sign_suite("crypto_sign");
}

#[test]
fn crypto_sign_ed25519_conversions_match() {
    unsafe {
        let (cpkb, _): (FnSize, FnSize) = pair("crypto_sign_ed25519_publickeybytes");
        let (cskb, _): (FnSize, FnSize) = pair("crypto_sign_ed25519_secretkeybytes");
        let (csdb, _): (FnSize, FnSize) = pair("crypto_sign_ed25519_seedbytes");
        let (ccb, _): (FnSize, FnSize) = pair("crypto_scalarmult_curve25519_bytes");
        let pkb = cpkb();
        let skb = cskb();
        let sdb = csdb();
        let cb = ccb();

        let (cskkp, _): (FnSeedKeypair, FnSeedKeypair) = pair("crypto_sign_ed25519_seed_keypair");
        let (cp2c, rp2c): (FnConvert, FnConvert) = pair("crypto_sign_ed25519_pk_to_curve25519");
        let (cs2c, rs2c): (FnConvert, FnConvert) = pair("crypto_sign_ed25519_sk_to_curve25519");
        let (cs2s, rs2s): (FnConvert, FnConvert) = pair("crypto_sign_ed25519_sk_to_seed");
        let (cs2p, rs2p): (FnConvert, FnConvert) = pair("crypto_sign_ed25519_sk_to_pk");

        let mut rng = Rng::new(0x6200);
        let mut pks: Vec<Vec<u8>> = Vec::new();
        let mut sks: Vec<Vec<u8>> = Vec::new();
        for _ in 0..8 {
            let seed = rng.vec(sdb);
            let mut pk = vec![0u8; pkb];
            let mut sk = vec![0u8; skb];
            assert_eq!(cskkp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0);
            pks.push(pk);
            sks.push(sk);
        }
        // plus degenerate / random keys
        pks.push(vec![0u8; pkb]);
        pks.push(vec![0xffu8; pkb]);
        for _ in 0..32 {
            pks.push(rng.vec(pkb));
        }
        sks.push(vec![0u8; skb]);
        sks.push(vec![0xffu8; skb]);
        for _ in 0..8 {
            sks.push(rng.vec(skb));
        }

        for pk in &pks {
            let mut co = vec![0xAAu8; cb + 8];
            let mut ro = vec![0xAAu8; cb + 8];
            let a = cp2c(co.as_mut_ptr(), pk.as_ptr());
            let b = rp2c(ro.as_mut_ptr(), pk.as_ptr());
            let tag = format!("pk_to_curve25519({})", hex(pk));
            assert_eq!(a, b, "{tag} return");
            assert_bytes_eq(&tag, &co, &ro);
        }
        for sk in &sks {
            for (name, cf, rf, olen) in [
                ("sk_to_curve25519", cs2c, rs2c, cb),
                ("sk_to_seed", cs2s, rs2s, sdb),
                ("sk_to_pk", cs2p, rs2p, pkb),
            ] {
                let mut co = vec![0xAAu8; olen + 8];
                let mut ro = vec![0xAAu8; olen + 8];
                let a = cf(co.as_mut_ptr(), sk.as_ptr());
                let b = rf(ro.as_mut_ptr(), sk.as_ptr());
                let tag = format!("{name}({})", hex(sk));
                assert_eq!(a, b, "{tag} return");
                assert_bytes_eq(&tag, &co, &ro);
            }
        }
    }
}

/// Multi-part (pre-hashed) signatures.
fn signph_suite(prefix: &str, sign_prefix: &str) {
    cmp_size(&format!("{prefix}_statebytes"));
    unsafe {
        let (csb, _): (FnSize, FnSize) = pair(&format!("{prefix}_statebytes"));
        let sb = csb();
        let (cb, _): (FnSize, FnSize) = pair(&format!("{sign_prefix}_bytes"));
        let (cpkb, _): (FnSize, FnSize) = pair(&format!("{sign_prefix}_publickeybytes"));
        let (cskb, _): (FnSize, FnSize) = pair(&format!("{sign_prefix}_secretkeybytes"));
        let (csdb, _): (FnSize, FnSize) = pair(&format!("{sign_prefix}_seedbytes"));
        let sigb = cb();
        let pkb = cpkb();
        let skb = cskb();
        let sdb = csdb();

        let (ci, ri): (FnPhInit, FnPhInit) = pair(&format!("{prefix}_init"));
        let (cu, ru): (FnPhUpdate, FnPhUpdate) = pair(&format!("{prefix}_update"));
        let (cfc, rfc): (FnPhFinalCreate, FnPhFinalCreate) =
            pair(&format!("{prefix}_final_create"));
        let (cfv, rfv): (FnPhFinalVerify, FnPhFinalVerify) =
            pair(&format!("{prefix}_final_verify"));
        let (cskkp, _): (FnSeedKeypair, FnSeedKeypair) =
            pair(&format!("{sign_prefix}_seed_keypair"));

        let mut rng = Rng::new(0x6300 + prefix.len() as u64);
        let msg = rng.vec(3001);
        let mut kps: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for _ in 0..3 {
            let seed = rng.vec(sdb);
            let mut pk = vec![0u8; pkb];
            let mut sk = vec![0u8; skb];
            assert_eq!(cskkp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0);
            kps.push((pk, sk));
        }

        for (pk, sk) in &kps {
            for &mlen in &[0usize, 1, 63, 64, 65, 127, 128, 129, 256, 1000, 3000] {
                for chunks in chunkings(mlen) {
                    let mut cst = AlignedBuf::new(sb, 0xA5);
                    let mut rst = AlignedBuf::new(sb, 0xA5);
                    let a = ci(cst.as_mut_ptr() as *mut c_void);
                    let b = ri(rst.as_mut_ptr() as *mut c_void);
                    assert_eq!(a, b, "{prefix}_init return");
                    assert_bytes_eq(
                        &format!("{prefix}_init state"),
                        cst.as_slice(),
                        rst.as_slice(),
                    );
                    let mut off = 0usize;
                    for &n in &chunks {
                        let a = cu(
                            cst.as_mut_ptr() as *mut c_void,
                            msg.as_ptr().add(off),
                            n as c_ulonglong,
                        );
                        let b = ru(
                            rst.as_mut_ptr() as *mut c_void,
                            msg.as_ptr().add(off),
                            n as c_ulonglong,
                        );
                        assert_eq!(a, b, "{prefix}_update return");
                        assert_bytes_eq(
                            &format!("{prefix} state mlen={mlen} chunk={n}"),
                            cst.as_slice(),
                            rst.as_slice(),
                        );
                        off += n;
                    }
                    // final_create on a copy of the state; final_verify on the
                    // other, so both libraries see equivalent state usage.
                    let mut cst2 = AlignedBuf::new(sb, 0xA5);
                    let mut rst2 = AlignedBuf::new(sb, 0xA5);
                    std::ptr::copy_nonoverlapping(
                        cst.as_ptr(),
                        cst2.as_mut_ptr(),
                        cst.as_slice().len(),
                    );
                    std::ptr::copy_nonoverlapping(
                        rst.as_ptr(),
                        rst2.as_mut_ptr(),
                        rst.as_slice().len(),
                    );

                    let mut csig = vec![0xAAu8; sigb + 8];
                    let mut rsig = vec![0xAAu8; sigb + 8];
                    let mut cl: c_ulonglong = 0xdead;
                    let mut rl: c_ulonglong = 0xdead;
                    let a = cfc(
                        cst.as_mut_ptr() as *mut c_void,
                        csig.as_mut_ptr(),
                        &mut cl,
                        sk.as_ptr(),
                    );
                    let b = rfc(
                        rst.as_mut_ptr() as *mut c_void,
                        rsig.as_mut_ptr(),
                        &mut rl,
                        sk.as_ptr(),
                    );
                    let tag = format!("{prefix}_final_create(mlen={mlen},chunks={chunks:?})");
                    assert_eq!(a, b, "{tag} return");
                    assert_eq!(cl, rl, "{tag} siglen");
                    assert_bytes_eq(&tag, &csig, &rsig);

                    let a = cfv(
                        cst2.as_mut_ptr() as *mut c_void,
                        csig.as_ptr(),
                        pk.as_ptr(),
                    );
                    let b = rfv(
                        rst2.as_mut_ptr() as *mut c_void,
                        csig.as_ptr(),
                        pk.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_final_verify good return");
                    assert_eq!(a, 0, "{prefix}_final_verify should succeed");
                }

                // verify with a bad signature
                let mut cst = AlignedBuf::new(sb, 0xA5);
                let mut rst = AlignedBuf::new(sb, 0xA5);
                ci(cst.as_mut_ptr() as *mut c_void);
                ri(rst.as_mut_ptr() as *mut c_void);
                cu(
                    cst.as_mut_ptr() as *mut c_void,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                );
                ru(
                    rst.as_mut_ptr() as *mut c_void,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                );
                let badsig = vec![0u8; sigb];
                let a = cfv(cst.as_mut_ptr() as *mut c_void, badsig.as_ptr(), pk.as_ptr());
                let b = rfv(rst.as_mut_ptr() as *mut c_void, badsig.as_ptr(), pk.as_ptr());
                assert_eq!(a, b, "{prefix}_final_verify zero-sig return");
            }
        }
    }
}

#[test]
fn crypto_sign_ed25519ph_matches() {
    signph_suite("crypto_sign_ed25519ph", "crypto_sign_ed25519");
}

#[test]
fn crypto_sign_multipart_generic_matches() {
    unsafe {
        // crypto_sign_init/update/final_* share the ed25519ph state
        let (csb, _): (FnSize, FnSize) = pair("crypto_sign_statebytes");
        let sb = csb();
        let (cb, _): (FnSize, FnSize) = pair("crypto_sign_bytes");
        let (cpkb, _): (FnSize, FnSize) = pair("crypto_sign_publickeybytes");
        let (cskb, _): (FnSize, FnSize) = pair("crypto_sign_secretkeybytes");
        let (csdb, _): (FnSize, FnSize) = pair("crypto_sign_seedbytes");
        let sigb = cb();
        let pkb = cpkb();
        let skb = cskb();
        let sdb = csdb();

        let (ci, ri): (FnPhInit, FnPhInit) = pair("crypto_sign_init");
        let (cu, ru): (FnPhUpdate, FnPhUpdate) = pair("crypto_sign_update");
        let (cfc, rfc): (FnPhFinalCreate, FnPhFinalCreate) = pair("crypto_sign_final_create");
        let (cfv, rfv): (FnPhFinalVerify, FnPhFinalVerify) = pair("crypto_sign_final_verify");
        let (cskkp, _): (FnSeedKeypair, FnSeedKeypair) = pair("crypto_sign_seed_keypair");

        let mut rng = Rng::new(0x6400);
        let msg = rng.vec(2001);
        let seed = rng.vec(sdb);
        let mut pk = vec![0u8; pkb];
        let mut sk = vec![0u8; skb];
        assert_eq!(cskkp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0);

        for &mlen in &[0usize, 1, 64, 128, 200, 1000, 2000] {
            let mut cst = AlignedBuf::new(sb, 0xA5);
            let mut rst = AlignedBuf::new(sb, 0xA5);
            assert_eq!(
                ci(cst.as_mut_ptr() as *mut c_void),
                ri(rst.as_mut_ptr() as *mut c_void),
                "crypto_sign_init return"
            );
            assert_bytes_eq("crypto_sign_init state", cst.as_slice(), rst.as_slice());
            cu(
                cst.as_mut_ptr() as *mut c_void,
                msg.as_ptr(),
                mlen as c_ulonglong,
            );
            ru(
                rst.as_mut_ptr() as *mut c_void,
                msg.as_ptr(),
                mlen as c_ulonglong,
            );
            assert_bytes_eq("crypto_sign_update state", cst.as_slice(), rst.as_slice());

            let mut cst2 = AlignedBuf::new(sb, 0xA5);
            let mut rst2 = AlignedBuf::new(sb, 0xA5);
            std::ptr::copy_nonoverlapping(cst.as_ptr(), cst2.as_mut_ptr(), cst.as_slice().len());
            std::ptr::copy_nonoverlapping(rst.as_ptr(), rst2.as_mut_ptr(), rst.as_slice().len());

            let mut csig = vec![0xAAu8; sigb + 8];
            let mut rsig = vec![0xAAu8; sigb + 8];
            let mut cl: c_ulonglong = 0xdead;
            let mut rl: c_ulonglong = 0xdead;
            let a = cfc(
                cst.as_mut_ptr() as *mut c_void,
                csig.as_mut_ptr(),
                &mut cl,
                sk.as_ptr(),
            );
            let b = rfc(
                rst.as_mut_ptr() as *mut c_void,
                rsig.as_mut_ptr(),
                &mut rl,
                sk.as_ptr(),
            );
            assert_eq!((a, cl), (b, rl), "crypto_sign_final_create mlen={mlen}");
            assert_bytes_eq(&format!("crypto_sign_final_create mlen={mlen}"), &csig, &rsig);

            let a = cfv(cst2.as_mut_ptr() as *mut c_void, csig.as_ptr(), pk.as_ptr());
            let b = rfv(rst2.as_mut_ptr() as *mut c_void, csig.as_ptr(), pk.as_ptr());
            assert_eq!(a, b, "crypto_sign_final_verify mlen={mlen}");
            assert_eq!(a, 0, "crypto_sign_final_verify should succeed");
        }
    }
}
