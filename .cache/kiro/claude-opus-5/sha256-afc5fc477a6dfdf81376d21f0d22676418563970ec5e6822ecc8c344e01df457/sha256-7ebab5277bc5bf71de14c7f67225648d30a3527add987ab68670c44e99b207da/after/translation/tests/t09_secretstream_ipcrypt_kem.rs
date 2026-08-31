//! secretstream (xchacha20poly1305), ipcrypt (deterministic / nd / ndx / pfx)
//! and the post-quantum KEMs (ML-KEM-768, X-Wing).
mod common;

use common::*;
use std::os::raw::{c_int, c_uchar, c_ulonglong, c_void};

type FnKeygen = unsafe extern "C" fn(*mut c_uchar);
type FnTag = unsafe extern "C" fn() -> c_uchar;
type FnInitPush = unsafe extern "C" fn(*mut c_void, *mut c_uchar, *const c_uchar) -> c_int;
type FnInitPull = unsafe extern "C" fn(*mut c_void, *const c_uchar, *const c_uchar) -> c_int;
type FnPush = unsafe extern "C" fn(
    *mut c_void,
    *mut c_uchar,
    *mut c_ulonglong,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    c_ulonglong,
    c_uchar,
) -> c_int;
type FnPull = unsafe extern "C" fn(
    *mut c_void,
    *mut c_uchar,
    *mut c_ulonglong,
    *mut c_uchar,
    *const c_uchar,
    c_ulonglong,
    *const c_uchar,
    c_ulonglong,
) -> c_int;
type FnRekey = unsafe extern "C" fn(*mut c_void);

#[test]
fn crypto_secretstream_xchacha20poly1305_matches() {
    let p = "crypto_secretstream_xchacha20poly1305";
    for s in [
        "abytes",
        "headerbytes",
        "keybytes",
        "messagebytes_max",
        "statebytes",
    ] {
        cmp_size(&format!("{p}_{s}"));
    }
    unsafe {
        for s in ["tag_message", "tag_push", "tag_rekey", "tag_final"] {
            let (c, r): (FnTag, FnTag) = pair(&format!("{p}_{s}"));
            assert_eq!(c(), r(), "{p}_{s}");
        }
        let g = |s: &str| -> usize {
            let (c, _): (FnSize, FnSize) = pair(&format!("{p}_{s}"));
            c()
        };
        let ab = g("abytes");
        let hb = g("headerbytes");
        let kb = g("keybytes");
        let sb = g("statebytes");

        let (ck, rk): (FnKeygen, FnKeygen) = pair(&format!("{p}_keygen"));
        let (cip, rip): (FnInitPush, FnInitPush) = pair(&format!("{p}_init_push"));
        let (cipl, ripl): (FnInitPull, FnInitPull) = pair(&format!("{p}_init_pull"));
        let (cpu, rpu): (FnPush, FnPush) = pair(&format!("{p}_push"));
        let (cpl, rpl): (FnPull, FnPull) = pair(&format!("{p}_pull"));
        let (cre, rre): (FnRekey, FnRekey) = pair(&format!("{p}_rekey"));

        let tag_msg: u8 = {
            let (c, _): (FnTag, FnTag) = pair(&format!("{p}_tag_message"));
            c()
        };
        let tag_push: u8 = {
            let (c, _): (FnTag, FnTag) = pair(&format!("{p}_tag_push"));
            c()
        };
        let tag_rekey: u8 = {
            let (c, _): (FnTag, FnTag) = pair(&format!("{p}_tag_rekey"));
            c()
        };
        let tag_final: u8 = {
            let (c, _): (FnTag, FnTag) = pair(&format!("{p}_tag_final"));
            c()
        };

        for _ in 0..4 {
            let mut a = vec![0xAAu8; kb + 8];
            let mut b = vec![0xAAu8; kb + 8];
            det_reset();
            ck(a.as_mut_ptr());
            det_reset();
            rk(b.as_mut_ptr());
            assert_bytes_eq(&format!("{p}_keygen"), &a, &b);
        }

        let mut rng = Rng::new(0x7000);
        let msg = rng.vec(3001);
        let ad = rng.vec(300);
        let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
        keys.push(rng.vec(kb));

        for key in &keys {
            // init_push consumes randomness for the header
            let mut cst = AlignedBuf::new(sb, 0xA5);
            let mut rst = AlignedBuf::new(sb, 0xA5);
            let mut chdr = vec![0xAAu8; hb + 8];
            let mut rhdr = vec![0xAAu8; hb + 8];
            det_reset();
            let a = cip(cst.as_mut_ptr() as *mut c_void, chdr.as_mut_ptr(), key.as_ptr());
            det_reset();
            let b = rip(rst.as_mut_ptr() as *mut c_void, rhdr.as_mut_ptr(), key.as_ptr());
            assert_eq!(a, b, "{p}_init_push return");
            assert_bytes_eq(&format!("{p}_init_push header"), &chdr, &rhdr);
            assert_bytes_eq(
                &format!("{p}_init_push state"),
                cst.as_slice(),
                rst.as_slice(),
            );

            // a scripted sequence of pushes covering every tag and rekeying
            let script: Vec<(usize, usize, u8)> = vec![
                (0, 0, tag_msg),
                (1, 0, tag_msg),
                (16, 1, tag_msg),
                (63, 16, tag_msg),
                (64, 32, tag_msg),
                (65, 0, tag_push),
                (128, 64, tag_msg),
                (129, 0, tag_rekey),
                (1000, 300, tag_msg),
                (0, 300, tag_msg),
                (3000, 7, tag_msg),
                (17, 0, tag_final),
            ];
            let mut cts: Vec<(Vec<u8>, usize, usize, u8)> = Vec::new();
            for (i, &(mlen, adlen, tag)) in script.iter().enumerate() {
                let adptr = if adlen == 0 {
                    std::ptr::null()
                } else {
                    ad.as_ptr()
                };
                let mut cc = vec![0xAAu8; mlen + ab + 8];
                let mut rc = vec![0xAAu8; mlen + ab + 8];
                let mut ccl: c_ulonglong = 0xdead;
                let mut rcl: c_ulonglong = 0xdead;
                let a = cpu(
                    cst.as_mut_ptr() as *mut c_void,
                    cc.as_mut_ptr(),
                    &mut ccl,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    adptr,
                    adlen as c_ulonglong,
                    tag,
                );
                let b = rpu(
                    rst.as_mut_ptr() as *mut c_void,
                    rc.as_mut_ptr(),
                    &mut rcl,
                    msg.as_ptr(),
                    mlen as c_ulonglong,
                    adptr,
                    adlen as c_ulonglong,
                    tag,
                );
                let t = format!("{p}_push[{i}](mlen={mlen},adlen={adlen},tag={tag})");
                assert_eq!(a, b, "{t} return");
                assert_eq!(ccl, rcl, "{t} clen");
                assert_bytes_eq(&t, &cc, &rc);
                assert_bytes_eq(&format!("{t} state"), cst.as_slice(), rst.as_slice());
                cts.push((cc[..ccl as usize].to_vec(), mlen, adlen, tag));
            }

            // explicit rekey must move both states identically
            cre(cst.as_mut_ptr() as *mut c_void);
            rre(rst.as_mut_ptr() as *mut c_void);
            assert_bytes_eq(&format!("{p}_rekey state"), cst.as_slice(), rst.as_slice());

            // pull side: replay the same sequence
            let mut cst = AlignedBuf::new(sb, 0xA5);
            let mut rst = AlignedBuf::new(sb, 0xA5);
            let a = cipl(
                cst.as_mut_ptr() as *mut c_void,
                chdr.as_ptr(),
                key.as_ptr(),
            );
            let b = ripl(
                rst.as_mut_ptr() as *mut c_void,
                chdr.as_ptr(),
                key.as_ptr(),
            );
            assert_eq!(a, b, "{p}_init_pull return");
            assert_bytes_eq(
                &format!("{p}_init_pull state"),
                cst.as_slice(),
                rst.as_slice(),
            );

            for (i, (ct, mlen, adlen, tag)) in cts.iter().enumerate() {
                let adptr = if *adlen == 0 {
                    std::ptr::null()
                } else {
                    ad.as_ptr()
                };
                let mut cm = vec![0xAAu8; mlen + 8];
                let mut rm = vec![0xAAu8; mlen + 8];
                let mut cml: c_ulonglong = 0xdead;
                let mut rml: c_ulonglong = 0xdead;
                let mut ctag: c_uchar = 0xAA;
                let mut rtag: c_uchar = 0xAA;
                let a = cpl(
                    cst.as_mut_ptr() as *mut c_void,
                    cm.as_mut_ptr(),
                    &mut cml,
                    &mut ctag,
                    ct.as_ptr(),
                    ct.len() as c_ulonglong,
                    adptr,
                    *adlen as c_ulonglong,
                );
                let b = rpl(
                    rst.as_mut_ptr() as *mut c_void,
                    rm.as_mut_ptr(),
                    &mut rml,
                    &mut rtag,
                    ct.as_ptr(),
                    ct.len() as c_ulonglong,
                    adptr,
                    *adlen as c_ulonglong,
                );
                let t = format!("{p}_pull[{i}](mlen={mlen},adlen={adlen})");
                assert_eq!(a, b, "{t} return");
                assert_eq!(cml, rml, "{t} mlen");
                assert_eq!(ctag, rtag, "{t} tag");
                assert_bytes_eq(&t, &cm, &rm);
                assert_bytes_eq(&format!("{t} state"), cst.as_slice(), rst.as_slice());
                assert_eq!(a, 0, "{t} should succeed");
                assert_eq!(ctag, *tag, "{t} tag value");
                assert_eq!(&cm[..*mlen], &msg[..*mlen], "{t} plaintext");
            }

            // tampered / truncated pulls, from a fresh pull state each time
            for (i, (ct, mlen, adlen, _)) in cts.iter().enumerate().take(4) {
                let adptr = if *adlen == 0 {
                    std::ptr::null()
                } else {
                    ad.as_ptr()
                };
                let mut bads: Vec<Vec<u8>> = vec![Vec::new(), ct[..ab - 1].to_vec()];
                let mut v = ct.clone();
                v[0] ^= 1;
                bads.push(v);
                let mut v = ct.clone();
                let n = v.len();
                v[n - 1] ^= 0x80;
                bads.push(v);
                bads.push(ct[..ct.len() - 1].to_vec());
                for bad in bads {
                    let mut cst = AlignedBuf::new(sb, 0xA5);
                    let mut rst = AlignedBuf::new(sb, 0xA5);
                    cipl(cst.as_mut_ptr() as *mut c_void, chdr.as_ptr(), key.as_ptr());
                    ripl(rst.as_mut_ptr() as *mut c_void, chdr.as_ptr(), key.as_ptr());
                    let mut cm = vec![0xAAu8; mlen + 16];
                    let mut rm = vec![0xAAu8; mlen + 16];
                    let mut cml: c_ulonglong = 0xdead;
                    let mut rml: c_ulonglong = 0xdead;
                    let mut ctag: c_uchar = 0xAA;
                    let mut rtag: c_uchar = 0xAA;
                    let a = cpl(
                        cst.as_mut_ptr() as *mut c_void,
                        cm.as_mut_ptr(),
                        &mut cml,
                        &mut ctag,
                        bad.as_ptr(),
                        bad.len() as c_ulonglong,
                        adptr,
                        *adlen as c_ulonglong,
                    );
                    let b = rpl(
                        rst.as_mut_ptr() as *mut c_void,
                        rm.as_mut_ptr(),
                        &mut rml,
                        &mut rtag,
                        bad.as_ptr(),
                        bad.len() as c_ulonglong,
                        adptr,
                        *adlen as c_ulonglong,
                    );
                    let t = format!("{p}_pull tampered[{i}](len={})", bad.len());
                    assert_eq!(a, b, "{t} return");
                    assert_eq!(cml, rml, "{t} mlen");
                    assert_eq!(ctag, rtag, "{t} tag");
                    assert_bytes_eq(&t, &cm, &rm);
                    assert_bytes_eq(&format!("{t} state"), cst.as_slice(), rst.as_slice());
                }
            }

            // NULL clen_p / mlen_p / tag_p
            let mut cst = AlignedBuf::new(sb, 0xA5);
            let mut rst = AlignedBuf::new(sb, 0xA5);
            det_reset();
            let mut chdr2 = vec![0u8; hb];
            cip(cst.as_mut_ptr() as *mut c_void, chdr2.as_mut_ptr(), key.as_ptr());
            det_reset();
            let mut rhdr2 = vec![0u8; hb];
            rip(rst.as_mut_ptr() as *mut c_void, rhdr2.as_mut_ptr(), key.as_ptr());
            let mut cc = vec![0xAAu8; 100 + ab + 8];
            let mut rc = vec![0xAAu8; 100 + ab + 8];
            let a = cpu(
                cst.as_mut_ptr() as *mut c_void,
                cc.as_mut_ptr(),
                std::ptr::null_mut(),
                msg.as_ptr(),
                100,
                std::ptr::null(),
                0,
                tag_msg,
            );
            let b = rpu(
                rst.as_mut_ptr() as *mut c_void,
                rc.as_mut_ptr(),
                std::ptr::null_mut(),
                msg.as_ptr(),
                100,
                std::ptr::null(),
                0,
                tag_msg,
            );
            assert_eq!(a, b, "{p}_push NULL clen_p return");
            assert_bytes_eq(&format!("{p}_push NULL clen_p"), &cc, &rc);

            let mut cst = AlignedBuf::new(sb, 0xA5);
            let mut rst = AlignedBuf::new(sb, 0xA5);
            cipl(cst.as_mut_ptr() as *mut c_void, chdr2.as_ptr(), key.as_ptr());
            ripl(rst.as_mut_ptr() as *mut c_void, chdr2.as_ptr(), key.as_ptr());
            let mut cm = vec![0xAAu8; 100 + 8];
            let mut rm = vec![0xAAu8; 100 + 8];
            let a = cpl(
                cst.as_mut_ptr() as *mut c_void,
                cm.as_mut_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                cc.as_ptr(),
                (100 + ab) as c_ulonglong,
                std::ptr::null(),
                0,
            );
            let b = rpl(
                rst.as_mut_ptr() as *mut c_void,
                rm.as_mut_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                cc.as_ptr(),
                (100 + ab) as c_ulonglong,
                std::ptr::null(),
                0,
            );
            assert_eq!(a, b, "{p}_pull NULL out-params return");
            assert_bytes_eq(&format!("{p}_pull NULL out-params"), &cm, &rm);

            // Note: `_pull` dereferences `m` unconditionally (it calls
            // chacha20_ietf_xor_ic straight into it), so unlike the AEAD
            // decrypt_detached APIs a NULL `m` is not a supported "verify only"
            // mode and is deliberately not exercised here.

            // arbitrary tag byte values on push
            for tag in [0u8, 1, 2, 3, 4, 0x0f, 0x7f, 0x80, 0xff] {
                let mut cst = AlignedBuf::new(sb, 0xA5);
                let mut rst = AlignedBuf::new(sb, 0xA5);
                det_reset();
                let mut h = vec![0u8; hb];
                cip(cst.as_mut_ptr() as *mut c_void, h.as_mut_ptr(), key.as_ptr());
                det_reset();
                let mut h2 = vec![0u8; hb];
                rip(rst.as_mut_ptr() as *mut c_void, h2.as_mut_ptr(), key.as_ptr());
                let mut cc = vec![0xAAu8; 64 + ab + 8];
                let mut rc = vec![0xAAu8; 64 + ab + 8];
                let mut ccl: c_ulonglong = 0xdead;
                let mut rcl: c_ulonglong = 0xdead;
                let a = cpu(
                    cst.as_mut_ptr() as *mut c_void,
                    cc.as_mut_ptr(),
                    &mut ccl,
                    msg.as_ptr(),
                    64,
                    std::ptr::null(),
                    0,
                    tag,
                );
                let b = rpu(
                    rst.as_mut_ptr() as *mut c_void,
                    rc.as_mut_ptr(),
                    &mut rcl,
                    msg.as_ptr(),
                    64,
                    std::ptr::null(),
                    0,
                    tag,
                );
                assert_eq!((a, ccl), (b, rcl), "{p}_push tag={tag}");
                assert_bytes_eq(&format!("{p}_push tag={tag}"), &cc, &rc);
                assert_bytes_eq(
                    &format!("{p}_push tag={tag} state"),
                    cst.as_slice(),
                    rst.as_slice(),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ipcrypt
// ---------------------------------------------------------------------------

type FnIp3 = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, *const c_uchar);
type FnIp4 = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, *const c_uchar, *const c_uchar);

#[test]
fn crypto_ipcrypt_matches() {
    for s in [
        "bytes",
        "keybytes",
        "nd_keybytes",
        "nd_tweakbytes",
        "nd_inputbytes",
        "nd_outputbytes",
        "ndx_keybytes",
        "ndx_tweakbytes",
        "ndx_inputbytes",
        "ndx_outputbytes",
        "pfx_keybytes",
        "pfx_bytes",
    ] {
        cmp_size(&format!("crypto_ipcrypt_{s}"));
    }
    unsafe {
        let g = |s: &str| -> usize {
            let (c, _): (FnSize, FnSize) = pair(&format!("crypto_ipcrypt_{s}"));
            c()
        };
        let mut rng = Rng::new(0x7100);

        // keygen variants
        for (name, kb) in [
            ("crypto_ipcrypt_keygen", g("keybytes")),
            ("crypto_ipcrypt_nd_keygen", g("nd_keybytes")),
            ("crypto_ipcrypt_ndx_keygen", g("ndx_keybytes")),
            ("crypto_ipcrypt_pfx_keygen", g("pfx_keybytes")),
        ] {
            let (c, r): (FnKeygen, FnKeygen) = pair(name);
            for _ in 0..4 {
                let mut a = vec![0xAAu8; kb + 8];
                let mut b = vec![0xAAu8; kb + 8];
                det_reset();
                c(a.as_mut_ptr());
                det_reset();
                r(b.as_mut_ptr());
                assert_bytes_eq(name, &a, &b);
            }
        }

        // deterministic ipcrypt: encrypt/decrypt (16-byte block, 16-byte key)
        {
            let bytes = g("bytes");
            let kb = g("keybytes");
            let (ce, re): (FnIp3, FnIp3) = pair("crypto_ipcrypt_encrypt");
            let (cd, rd): (FnIp3, FnIp3) = pair("crypto_ipcrypt_decrypt");
            let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
            for _ in 0..4 {
                keys.push(rng.vec(kb));
            }
            let mut ins: Vec<Vec<u8>> = vec![vec![0u8; bytes], vec![0xffu8; bytes]];
            for bit in 0..(bytes * 8) {
                let mut v = vec![0u8; bytes];
                v[bit / 8] = 1 << (bit % 8);
                ins.push(v);
            }
            for _ in 0..32 {
                ins.push(rng.vec(bytes));
            }
            for key in &keys {
                for inb in &ins {
                    let mut co = vec![0xAAu8; bytes + 8];
                    let mut ro = vec![0xAAu8; bytes + 8];
                    ce(co.as_mut_ptr(), inb.as_ptr(), key.as_ptr());
                    re(ro.as_mut_ptr(), inb.as_ptr(), key.as_ptr());
                    assert_bytes_eq(
                        &format!("crypto_ipcrypt_encrypt(in={},k={})", hex(inb), hex(key)),
                        &co,
                        &ro,
                    );
                    let mut cb = vec![0xAAu8; bytes + 8];
                    let mut rb = vec![0xAAu8; bytes + 8];
                    cd(cb.as_mut_ptr(), co.as_ptr(), key.as_ptr());
                    rd(rb.as_mut_ptr(), co.as_ptr(), key.as_ptr());
                    assert_bytes_eq("crypto_ipcrypt_decrypt", &cb, &rb);
                    assert_eq!(&cb[..bytes], &inb[..], "ipcrypt roundtrip");
                    // decrypt of arbitrary input too
                    let mut cb = vec![0xAAu8; bytes + 8];
                    let mut rb = vec![0xAAu8; bytes + 8];
                    cd(cb.as_mut_ptr(), inb.as_ptr(), key.as_ptr());
                    rd(rb.as_mut_ptr(), inb.as_ptr(), key.as_ptr());
                    assert_bytes_eq("crypto_ipcrypt_decrypt raw", &cb, &rb);
                }
            }
        }

        // pfx: prefix-preserving, same shape as the deterministic variant
        {
            let bytes = g("pfx_bytes");
            let kb = g("pfx_keybytes");
            let (ce, re): (FnIp3, FnIp3) = pair("crypto_ipcrypt_pfx_encrypt");
            let (cd, rd): (FnIp3, FnIp3) = pair("crypto_ipcrypt_pfx_decrypt");
            let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
            for _ in 0..4 {
                keys.push(rng.vec(kb));
            }
            let mut ins: Vec<Vec<u8>> = vec![vec![0u8; bytes], vec![0xffu8; bytes]];
            for bit in 0..(bytes * 8) {
                let mut v = vec![0u8; bytes];
                v[bit / 8] = 1 << (bit % 8);
                ins.push(v);
            }
            // IPv4-mapped forms exercise the v4/v6 branch
            for i in 0..16u8 {
                let mut v = vec![0u8; bytes];
                v[10] = 0xff;
                v[11] = 0xff;
                v[12] = 192;
                v[13] = 168;
                v[14] = i;
                v[15] = i.wrapping_mul(7);
                ins.push(v);
            }
            for _ in 0..32 {
                ins.push(rng.vec(bytes));
            }
            for key in &keys {
                for inb in &ins {
                    let mut co = vec![0xAAu8; bytes + 8];
                    let mut ro = vec![0xAAu8; bytes + 8];
                    ce(co.as_mut_ptr(), inb.as_ptr(), key.as_ptr());
                    re(ro.as_mut_ptr(), inb.as_ptr(), key.as_ptr());
                    assert_bytes_eq(
                        &format!("crypto_ipcrypt_pfx_encrypt(in={})", hex(inb)),
                        &co,
                        &ro,
                    );
                    let mut cb = vec![0xAAu8; bytes + 8];
                    let mut rb = vec![0xAAu8; bytes + 8];
                    cd(cb.as_mut_ptr(), co.as_ptr(), key.as_ptr());
                    rd(rb.as_mut_ptr(), co.as_ptr(), key.as_ptr());
                    assert_bytes_eq("crypto_ipcrypt_pfx_decrypt", &cb, &rb);
                    let mut cb = vec![0xAAu8; bytes + 8];
                    let mut rb = vec![0xAAu8; bytes + 8];
                    cd(cb.as_mut_ptr(), inb.as_ptr(), key.as_ptr());
                    rd(rb.as_mut_ptr(), inb.as_ptr(), key.as_ptr());
                    assert_bytes_eq("crypto_ipcrypt_pfx_decrypt raw", &cb, &rb);
                }
            }
        }

        // nd / ndx: tweakable, encrypt takes an explicit tweak
        for variant in ["nd", "ndx"] {
            let inb_len = g(&format!("{variant}_inputbytes"));
            let outb_len = g(&format!("{variant}_outputbytes"));
            let tb = g(&format!("{variant}_tweakbytes"));
            let kb = g(&format!("{variant}_keybytes"));
            let (ce, re): (FnIp4, FnIp4) = pair(&format!("crypto_ipcrypt_{variant}_encrypt"));
            let (cd, rd): (FnIp3, FnIp3) = pair(&format!("crypto_ipcrypt_{variant}_decrypt"));
            let mut keys: Vec<Vec<u8>> = vec![vec![0u8; kb], vec![0xffu8; kb]];
            for _ in 0..3 {
                keys.push(rng.vec(kb));
            }
            let mut tweaks: Vec<Vec<u8>> = vec![vec![0u8; tb], vec![0xffu8; tb]];
            for _ in 0..3 {
                tweaks.push(rng.vec(tb));
            }
            let mut ins: Vec<Vec<u8>> = vec![vec![0u8; inb_len], vec![0xffu8; inb_len]];
            for bit in 0..(inb_len * 8) {
                let mut v = vec![0u8; inb_len];
                v[bit / 8] = 1 << (bit % 8);
                ins.push(v);
            }
            for _ in 0..16 {
                ins.push(rng.vec(inb_len));
            }
            for key in &keys {
                for tweak in &tweaks {
                    for inb in &ins {
                        let mut co = vec![0xAAu8; outb_len + 8];
                        let mut ro = vec![0xAAu8; outb_len + 8];
                        ce(co.as_mut_ptr(), inb.as_ptr(), tweak.as_ptr(), key.as_ptr());
                        re(ro.as_mut_ptr(), inb.as_ptr(), tweak.as_ptr(), key.as_ptr());
                        assert_bytes_eq(
                            &format!("crypto_ipcrypt_{variant}_encrypt(in={})", hex(inb)),
                            &co,
                            &ro,
                        );
                        let mut cb = vec![0xAAu8; inb_len + 8];
                        let mut rb = vec![0xAAu8; inb_len + 8];
                        cd(cb.as_mut_ptr(), co.as_ptr(), key.as_ptr());
                        rd(rb.as_mut_ptr(), co.as_ptr(), key.as_ptr());
                        assert_bytes_eq(&format!("crypto_ipcrypt_{variant}_decrypt"), &cb, &rb);
                        assert_eq!(&cb[..inb_len], &inb[..], "{variant} roundtrip");
                    }
                }
            }
            // decrypt of arbitrary (non-ciphertext) inputs
            for key in keys.iter().take(2) {
                for _ in 0..32 {
                    let raw = rng.vec(outb_len);
                    let mut cb = vec![0xAAu8; inb_len + 8];
                    let mut rb = vec![0xAAu8; inb_len + 8];
                    cd(cb.as_mut_ptr(), raw.as_ptr(), key.as_ptr());
                    rd(rb.as_mut_ptr(), raw.as_ptr(), key.as_ptr());
                    assert_bytes_eq(&format!("crypto_ipcrypt_{variant}_decrypt raw"), &cb, &rb);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// KEMs
// ---------------------------------------------------------------------------

type FnSeedKeypair = unsafe extern "C" fn(*mut c_uchar, *mut c_uchar, *const c_uchar) -> c_int;
type FnKeypair = unsafe extern "C" fn(*mut c_uchar, *mut c_uchar) -> c_int;
type FnEnc = unsafe extern "C" fn(*mut c_uchar, *mut c_uchar, *const c_uchar) -> c_int;
type FnEncDet =
    unsafe extern "C" fn(*mut c_uchar, *mut c_uchar, *const c_uchar, *const c_uchar) -> c_int;
type FnDec = unsafe extern "C" fn(*mut c_uchar, *const c_uchar, *const c_uchar) -> c_int;

fn kem_suite(prefix: &str, enc_det_seedlen: usize) {
    for s in [
        "publickeybytes",
        "secretkeybytes",
        "ciphertextbytes",
        "sharedsecretbytes",
        "seedbytes",
    ] {
        cmp_size(&format!("{prefix}_{s}"));
    }
    unsafe {
        let g = |s: &str| -> usize {
            let (c, _): (FnSize, FnSize) = pair(&format!("{prefix}_{s}"));
            c()
        };
        let pkb = g("publickeybytes");
        let skb = g("secretkeybytes");
        let ctb = g("ciphertextbytes");
        let ssb = g("sharedsecretbytes");
        let sdb = g("seedbytes");

        let (csk, rsk): (FnSeedKeypair, FnSeedKeypair) = pair(&format!("{prefix}_seed_keypair"));
        let (ckp, rkp): (FnKeypair, FnKeypair) = pair(&format!("{prefix}_keypair"));
        let (ce, re): (FnEnc, FnEnc) = pair(&format!("{prefix}_enc"));
        let (cd, rd): (FnDec, FnDec) = pair(&format!("{prefix}_dec"));

        let mut rng = Rng::new(0x7200 + prefix.len() as u64);
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

        // enc: consumes randomness -> reset before each side
        let mut pks: Vec<Vec<u8>> = kps.iter().map(|(p, _)| p.clone()).collect();
        pks.push(vec![0u8; pkb]);
        pks.push(vec![0xffu8; pkb]);
        pks.push(rng.vec(pkb));
        for pk in &pks {
            let mut cct = vec![0xAAu8; ctb + 8];
            let mut rct = vec![0xAAu8; ctb + 8];
            let mut css = vec![0xAAu8; ssb + 8];
            let mut rss = vec![0xAAu8; ssb + 8];
            det_reset();
            let a = ce(cct.as_mut_ptr(), css.as_mut_ptr(), pk.as_ptr());
            det_reset();
            let b = re(rct.as_mut_ptr(), rss.as_mut_ptr(), pk.as_ptr());
            assert_eq!(a, b, "{prefix}_enc return");
            assert_bytes_eq(&format!("{prefix}_enc ct"), &cct, &rct);
            assert_bytes_eq(&format!("{prefix}_enc ss"), &css, &rss);
        }

        // deterministic enc
        if enc_det_seedlen > 0 {
            let (ced, red): (FnEncDet, FnEncDet) = pair(&format!("{prefix}_enc_deterministic"));
            let mut dseeds: Vec<Vec<u8>> = vec![
                vec![0u8; enc_det_seedlen],
                vec![0xffu8; enc_det_seedlen],
            ];
            for _ in 0..6 {
                dseeds.push(rng.vec(enc_det_seedlen));
            }
            for pk in &pks {
                for ds in &dseeds {
                    let mut cct = vec![0xAAu8; ctb + 8];
                    let mut rct = vec![0xAAu8; ctb + 8];
                    let mut css = vec![0xAAu8; ssb + 8];
                    let mut rss = vec![0xAAu8; ssb + 8];
                    let a = ced(
                        cct.as_mut_ptr(),
                        css.as_mut_ptr(),
                        pk.as_ptr(),
                        ds.as_ptr(),
                    );
                    let b = red(
                        rct.as_mut_ptr(),
                        rss.as_mut_ptr(),
                        pk.as_ptr(),
                        ds.as_ptr(),
                    );
                    assert_eq!(a, b, "{prefix}_enc_deterministic return");
                    assert_bytes_eq(&format!("{prefix}_enc_deterministic ct"), &cct, &rct);
                    assert_bytes_eq(&format!("{prefix}_enc_deterministic ss"), &css, &rss);
                }
            }

            // dec, including implicit-rejection paths on tampered ciphertexts
            for (pk, sk) in &kps {
                for ds in dseeds.iter().take(3) {
                    let mut ct = vec![0u8; ctb];
                    let mut ss = vec![0u8; ssb];
                    let a = ced(ct.as_mut_ptr(), ss.as_mut_ptr(), pk.as_ptr(), ds.as_ptr());
                    assert_eq!(a, 0, "{prefix}_enc_deterministic should succeed");

                    let mut css = vec![0xAAu8; ssb + 8];
                    let mut rss = vec![0xAAu8; ssb + 8];
                    let a = cd(css.as_mut_ptr(), ct.as_ptr(), sk.as_ptr());
                    let b = rd(rss.as_mut_ptr(), ct.as_ptr(), sk.as_ptr());
                    assert_eq!(a, b, "{prefix}_dec return");
                    assert_bytes_eq(&format!("{prefix}_dec ss"), &css, &rss);
                    assert_eq!(&css[..ssb], &ss[..], "{prefix} enc/dec agreement");

                    // tampered ciphertexts must yield identical (rejection) secrets
                    for pos in [0usize, 1, ctb / 2, ctb - 1] {
                        let mut bad = ct.clone();
                        bad[pos] ^= 0x01;
                        let mut css = vec![0xAAu8; ssb + 8];
                        let mut rss = vec![0xAAu8; ssb + 8];
                        let a = cd(css.as_mut_ptr(), bad.as_ptr(), sk.as_ptr());
                        let b = rd(rss.as_mut_ptr(), bad.as_ptr(), sk.as_ptr());
                        assert_eq!(a, b, "{prefix}_dec tampered@{pos} return");
                        assert_bytes_eq(&format!("{prefix}_dec tampered@{pos}"), &css, &rss);
                    }
                    // all-zero and all-ones ciphertexts
                    for bad in [vec![0u8; ctb], vec![0xffu8; ctb]] {
                        let mut css = vec![0xAAu8; ssb + 8];
                        let mut rss = vec![0xAAu8; ssb + 8];
                        let a = cd(css.as_mut_ptr(), bad.as_ptr(), sk.as_ptr());
                        let b = rd(rss.as_mut_ptr(), bad.as_ptr(), sk.as_ptr());
                        assert_eq!(a, b, "{prefix}_dec degenerate return");
                        assert_bytes_eq(&format!("{prefix}_dec degenerate"), &css, &rss);
                    }
                }
            }
        }

        // dec with corrupted secret keys
        for (_, sk) in kps.iter().take(2) {
            let ct = rng.vec(ctb);
            let mut badsks: Vec<Vec<u8>> = vec![vec![0u8; skb], vec![0xffu8; skb]];
            let mut v = sk.clone();
            v[0] ^= 1;
            badsks.push(v);
            let mut v = sk.clone();
            v[skb - 1] ^= 0x80;
            badsks.push(v);
            for badsk in &badsks {
                let mut css = vec![0xAAu8; ssb + 8];
                let mut rss = vec![0xAAu8; ssb + 8];
                let a = cd(css.as_mut_ptr(), ct.as_ptr(), badsk.as_ptr());
                let b = rd(rss.as_mut_ptr(), ct.as_ptr(), badsk.as_ptr());
                assert_eq!(a, b, "{prefix}_dec badsk return");
                assert_bytes_eq(&format!("{prefix}_dec badsk"), &css, &rss);
            }
        }
    }
}

#[test]
fn crypto_kem_mlkem768_matches() {
    kem_suite("crypto_kem_mlkem768", 32);
}

#[test]
fn crypto_kem_xwing_matches() {
    kem_suite("crypto_kem_xwing", 64);
}

#[test]
fn crypto_kem_generic_matches() {
    cmp_cstr("crypto_kem_primitive");
    kem_suite("crypto_kem", 0);
}
