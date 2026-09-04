//! Phase B, CONFIGS.md rows 63-68: the NIST CTR_DRBG of `app/src/rng.c`.
//!
//! `rng.c` is compiled into `libsphincs_core_det.so` in every configuration, so
//! these rows run unchanged under the `urandom` feature too — the only thing the
//! feature switches is which provider `randombytes` itself uses, which is why
//! row 63's output comparison is gated.

mod common;

use common::*;

type Randombytes = unsafe extern "C" fn(*mut u8, u64) -> i32;
type RandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);
type Aes256Ecb = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type DrbgUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type SeedexpanderInit = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, *mut u8, u64) -> i32;
type Seedexpander = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, u64) -> i32;

const DRBG_DETERMINISTIC: bool = !cfg!(feature = "urandom");

#[test]
fn row63_randombytes_init_and_draw() {
    let libs = load();
    let (ic, ir) = libs.pair::<RandombytesInit>("randombytes_init");
    let (rc, rr) = libs.pair::<Randombytes>("randombytes");
    let dc = libs.c::<*mut Aes256CtrDrbgStruct>("DRBG_ctx");
    let dr = libs.r::<*mut Aes256CtrDrbgStruct>("DRBG_ctx");
    let mut rng = Rng::new(63);

    for pers_mode in 0..2 {
        for trial in 0..4 {
            let mut ec = [0u8; 48];
            rng.fill(&mut ec);
            let mut er = ec;
            let mut pc = [0u8; 48];
            rng.fill(&mut pc);
            let mut pr = pc;
            unsafe {
                if pers_mode == 0 {
                    ic(ec.as_mut_ptr(), core::ptr::null_mut());
                    ir(er.as_mut_ptr(), core::ptr::null_mut());
                } else {
                    ic(ec.as_mut_ptr(), pc.as_mut_ptr());
                    ir(er.as_mut_ptr(), pr.as_mut_ptr());
                }
                eq(
                    &format!("DRBG_ctx after randombytes_init(pers={pers_mode})"),
                    (**dc).as_bytes(),
                    (**dr).as_bytes(),
                );
            }
            // Sequential draws: the DRBG is stateful, so every call has to keep
            // the two sides in step.
            for &xlen in &[0usize, 1, 15, 16, 17, 31, 32, 33, 47, 48, 100, 1000] {
                let mut a = vec![0xA5u8; xlen + 8];
                let mut b = vec![0xA5u8; xlen + 8];
                let (ca, cb) = unsafe {
                    (
                        rc(a.as_mut_ptr(), xlen as u64),
                        rr(b.as_mut_ptr(), xlen as u64),
                    )
                };
                assert_eq!(ca, 0, "C randombytes returned {ca}");
                if DRBG_DETERMINISTIC {
                    assert_eq!(cb, 0, "Rust randombytes returned {cb}");
                    eq(&format!("randombytes(xlen={xlen}, trial={trial})"), &a, &b);
                    unsafe {
                        eq(
                            &format!("DRBG_ctx after randombytes({xlen})"),
                            (**dc).as_bytes(),
                            (**dr).as_bytes(),
                        );
                    }
                }
            }
        }
    }
    // extremes
    let mut z = [0u8; 48];
    let mut o = [0xFFu8; 48];
    unsafe {
        ic(z.as_mut_ptr(), core::ptr::null_mut());
        ir(z.as_mut_ptr(), core::ptr::null_mut());
        eq("DRBG_ctx zero entropy", (**dc).as_bytes(), (**dr).as_bytes());
        ic(o.as_mut_ptr(), o.as_mut_ptr());
        ir(o.as_mut_ptr(), o.as_mut_ptr());
        eq("DRBG_ctx ones entropy+pers", (**dc).as_bytes(), (**dr).as_bytes());
    }
}

#[test]
fn row64_randombytes_carry_chain() {
    if !DRBG_DETERMINISTIC {
        eprintln!("[{}] row64 skipped: urandom provider", tag());
        return;
    }
    let libs = load();
    let (ic, ir) = libs.pair::<RandombytesInit>("randombytes_init");
    let (rc, rr) = libs.pair::<Randombytes>("randombytes");
    let dc = libs.c::<*mut Aes256CtrDrbgStruct>("DRBG_ctx");
    let dr = libs.r::<*mut Aes256CtrDrbgStruct>("DRBG_ctx");
    let mut ent = [7u8; 48];
    unsafe {
        ic(ent.as_mut_ptr(), core::ptr::null_mut());
        ir(ent.as_mut_ptr(), core::ptr::null_mut());
        // Force V to all-ones on both sides so the increment loop has to carry
        // through all 16 bytes, then draw.
        (**dc).V = [0xFFu8; 16];
        (**dr).V = [0xFFu8; 16];
        for k in 0..8 {
            let mut a = [0u8; 64];
            let mut b = [0u8; 64];
            rc(a.as_mut_ptr(), 64);
            rr(b.as_mut_ptr(), 64);
            eq(&format!("randombytes after V=FF.. (k={k})"), &a, &b);
            eq("DRBG_ctx after carry", (**dc).as_bytes(), (**dr).as_bytes());
            // and again with only the low bytes at FF
            (**dc).V = [0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xFF];
            (**dr).V = (**dc).V;
        }
    }
}

#[test]
fn row65_aes256_ecb() {
    let libs = load();
    let (fc, fr) = libs.pair::<Aes256Ecb>("AES256_ECB");
    let mut rng = Rng::new(65);
    let mut cases: Vec<([u8; 32], [u8; 16])> = vec![
        ([0u8; 32], [0u8; 16]),
        ([0xFFu8; 32], [0xFFu8; 16]),
        ([0u8; 32], [0xFFu8; 16]),
        ([0xFFu8; 32], [0u8; 16]),
    ];
    for _ in 0..256 {
        let mut k = [0u8; 32];
        let mut c = [0u8; 16];
        rng.fill(&mut k);
        rng.fill(&mut c);
        cases.push((k, c));
    }
    for (k, c) in cases {
        let mut ka = k;
        let mut kb = k;
        let mut ca = c;
        let mut cb = c;
        let mut a = [0xA5u8; 16];
        let mut b = [0xA5u8; 16];
        unsafe {
            fc(ka.as_mut_ptr(), ca.as_mut_ptr(), a.as_mut_ptr());
            fr(kb.as_mut_ptr(), cb.as_mut_ptr(), b.as_mut_ptr());
        }
        eq("AES256_ECB out", &a, &b);
        eq("AES256_ECB key untouched", &ka, &kb);
        eq("AES256_ECB ctr untouched", &ca, &cb);
    }
}

#[test]
fn row66_drbg_update() {
    let libs = load();
    let (fc, fr) = libs.pair::<DrbgUpdate>("AES256_CTR_DRBG_Update");
    let mut rng = Rng::new(66);
    for pd_null in [true, false] {
        for vmode in 0..3 {
            for _ in 0..64 {
                let mut key = [0u8; 32];
                rng.fill(&mut key);
                let v: [u8; 16] = match vmode {
                    0 => [0u8; 16],
                    1 => [0xFFu8; 16],
                    _ => {
                        let mut t = [0u8; 16];
                        rng.fill(&mut t);
                        t
                    }
                };
                let mut pd = [0u8; 48];
                rng.fill(&mut pd);

                let mut ka = key;
                let mut kb = key;
                let mut va = v;
                let mut vb = v;
                let mut pda = pd;
                let mut pdb = pd;
                unsafe {
                    if pd_null {
                        fc(core::ptr::null_mut(), ka.as_mut_ptr(), va.as_mut_ptr());
                        fr(core::ptr::null_mut(), kb.as_mut_ptr(), vb.as_mut_ptr());
                    } else {
                        fc(pda.as_mut_ptr(), ka.as_mut_ptr(), va.as_mut_ptr());
                        fr(pdb.as_mut_ptr(), kb.as_mut_ptr(), vb.as_mut_ptr());
                    }
                }
                eq(
                    &format!("AES256_CTR_DRBG_Update Key (null={pd_null}, v={vmode})"),
                    &ka,
                    &kb,
                );
                eq(
                    &format!("AES256_CTR_DRBG_Update V (null={pd_null}, v={vmode})"),
                    &va,
                    &vb,
                );
                eq("AES256_CTR_DRBG_Update provided_data untouched", &pda, &pdb);
            }
        }
    }
}

#[test]
fn row67_seedexpander() {
    let libs = load();
    let (ic, ir) = libs.pair::<SeedexpanderInit>("seedexpander_init");
    let (ec, er) = libs.pair::<Seedexpander>("seedexpander");
    let mut rng = Rng::new(67);

    for &maxlen in &[1u64, 2, 16, 17, 33, 4096, 0xFFFF_FFFF] {
        let mut seed = [0u8; 32];
        rng.fill(&mut seed);
        let mut div = [0u8; 8];
        rng.fill(&mut div);
        let mut ca = AesXofStruct::zeroed();
        let mut cb = AesXofStruct::zeroed();
        // pre-fill with a sentinel so an incomplete init is visible
        for (a, b) in ca.buffer.iter_mut().zip(cb.buffer.iter_mut()) {
            *a = 0x5A;
            *b = 0x5A;
        }
        let (ra, rb) = unsafe {
            (
                ic(&mut ca, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen),
                ir(&mut cb, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen),
            )
        };
        assert_eq!(ra, rb, "seedexpander_init return (maxlen={maxlen})");
        assert_eq!(ra, 0, "seedexpander_init(maxlen={maxlen}) should succeed");
        eq(
            &format!("AES_XOF_struct after init(maxlen={maxlen})"),
            ca.as_bytes(),
            cb.as_bytes(),
        );

        let mut remaining = maxlen;
        for &xlen in &[1u64, 15, 16, 17, 32, 100, 3, 7] {
            if xlen >= remaining {
                continue;
            }
            let mut a = vec![0xA5u8; xlen as usize + 8];
            let mut b = vec![0xA5u8; xlen as usize + 8];
            let (sa, sb) = unsafe { (ec(&mut ca, a.as_mut_ptr(), xlen), er(&mut cb, b.as_mut_ptr(), xlen)) };
            assert_eq!(sa, sb, "seedexpander return (xlen={xlen})");
            assert_eq!(sa, 0);
            eq(&format!("seedexpander out (maxlen={maxlen}, xlen={xlen})"), &a, &b);
            eq(
                &format!("AES_XOF_struct after seedexpander({xlen})"),
                ca.as_bytes(),
                cb.as_bytes(),
            );
            remaining -= xlen;
        }
    }
}

#[test]
fn row68_seedexpander_counter_carry() {
    let libs = load();
    let (ic, ir) = libs.pair::<SeedexpanderInit>("seedexpander_init");
    let (ec, er) = libs.pair::<Seedexpander>("seedexpander");
    let mut seed = [3u8; 32];
    let mut div = [9u8; 8];
    let mut ca = AesXofStruct::zeroed();
    let mut cb = AesXofStruct::zeroed();
    unsafe {
        assert_eq!(ic(&mut ca, seed.as_mut_ptr(), div.as_mut_ptr(), 0xFFFF_FFFF), 0);
        assert_eq!(ir(&mut cb, seed.as_mut_ptr(), div.as_mut_ptr(), 0xFFFF_FFFF), 0);
    }
    // Drive the ctr[12..16] carry chain through each of its stages.
    for tail in [
        [0xFFu8, 0xFF, 0xFF, 0xFF],
        [0x00, 0xFF, 0xFF, 0xFF],
        [0x00, 0x00, 0xFF, 0xFF],
        [0x00, 0x00, 0x00, 0xFF],
    ] {
        ca.ctr[12..16].copy_from_slice(&tail);
        cb.ctr[12..16].copy_from_slice(&tail);
        ca.buffer_pos = 16;
        cb.buffer_pos = 16;
        for &xlen in &[17u64, 33, 64, 129] {
            let mut a = vec![0u8; xlen as usize];
            let mut b = vec![0u8; xlen as usize];
            let (sa, sb) = unsafe { (ec(&mut ca, a.as_mut_ptr(), xlen), er(&mut cb, b.as_mut_ptr(), xlen)) };
            assert_eq!(sa, sb);
            eq(&format!("seedexpander carry tail={tail:?} xlen={xlen}"), &a, &b);
            eq("AES_XOF_struct after carry", ca.as_bytes(), cb.as_bytes());
        }
    }
}
