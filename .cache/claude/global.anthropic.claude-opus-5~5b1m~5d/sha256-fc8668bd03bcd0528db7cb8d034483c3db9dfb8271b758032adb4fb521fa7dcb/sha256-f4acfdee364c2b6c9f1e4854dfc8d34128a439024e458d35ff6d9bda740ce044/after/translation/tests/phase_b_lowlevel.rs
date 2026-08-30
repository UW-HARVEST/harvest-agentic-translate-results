//! Phase B rows 1-12: utils, address setters, ctx initialisation, prf_addr,
//! thash across every input width the C code special-cases.
mod common;
use common::*;
use libloading::Library;

type FUll = unsafe extern "C" fn(*mut u8, u32, u64);
type FU32 = unsafe extern "C" fn(*mut u8, u32);
type FB2U = unsafe extern "C" fn(*const u8, u32) -> u64;
type FAddrU32 = unsafe extern "C" fn(*mut u32, u32);
type FAddrU64 = unsafe extern "C" fn(*mut u32, u64);
type FAddrCopy = unsafe extern "C" fn(*mut u32, *const u32);
type FPrf = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
type FThash = unsafe extern "C" fn(*mut u8, *const u8, std::ffi::c_uint, *const u8, *mut u32);

// ---------- row 1 ----------
#[test]
fn b01_ull_to_bytes() {
    let p = pair();
    p.check_config();
    let mut rng = Rng::new(SEED ^ 1);
    unsafe {
        let cf = sym!(p.c, b"SPX_ull_to_bytes\0", FUll);
        let rf = sym!(p.r, b"SPX_ull_to_bytes\0", FUll);
        for outlen in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16] {
            for _ in 0..64 {
                let v = rng.next_u64();
                let mut cb = vec![0xAAu8; 32];
                let mut rb = vec![0xAAu8; 32];
                cf(cb.as_mut_ptr(), outlen, v);
                rf(rb.as_mut_ptr(), outlen, v);
                eqb(&format!("ull_to_bytes outlen={outlen} v={v:#x}"), &cb, &rb);
            }
            // extremes
            for v in [0u64, 1, u64::MAX, 0x00FF_00FF_00FF_00FF] {
                let mut cb = vec![0x55u8; 32];
                let mut rb = vec![0x55u8; 32];
                cf(cb.as_mut_ptr(), outlen, v);
                rf(rb.as_mut_ptr(), outlen, v);
                eqb(&format!("ull_to_bytes outlen={outlen} v={v:#x}"), &cb, &rb);
            }
        }
    }
}

// ---------- row 2 ----------
#[test]
fn b02_u32_to_bytes() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 2);
    unsafe {
        let cf = sym!(p.c, b"SPX_u32_to_bytes\0", FU32);
        let rf = sym!(p.r, b"SPX_u32_to_bytes\0", FU32);
        let mut vals: Vec<u32> = vec![0, 1, 0xFF, 0x100, 0xFFFF, 0xFFFF_FFFF, 0x8000_0000];
        for _ in 0..128 {
            vals.push(rng.next_u32());
        }
        for v in vals {
            let mut cb = [0xAAu8; 8];
            let mut rb = [0xAAu8; 8];
            cf(cb.as_mut_ptr(), v);
            rf(rb.as_mut_ptr(), v);
            eqb(&format!("u32_to_bytes {v:#x}"), &cb, &rb);
        }
    }
}

// ---------- row 3 ----------
#[test]
fn b03_bytes_to_ull() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 3);
    unsafe {
        let cf = sym!(p.c, b"SPX_bytes_to_ull\0", FB2U);
        let rf = sym!(p.r, b"SPX_bytes_to_ull\0", FB2U);
        for inlen in 0u32..=8 {
            for _ in 0..64 {
                let b = rng.bytes(16);
                eqv(
                    &format!("bytes_to_ull inlen={inlen}"),
                    cf(b.as_ptr(), inlen),
                    rf(b.as_ptr(), inlen),
                );
            }
            for pat in [0x00u8, 0xff, 0x80, 0x01] {
                let b = vec![pat; 16];
                eqv(
                    &format!("bytes_to_ull inlen={inlen} pat={pat:#x}"),
                    cf(b.as_ptr(), inlen),
                    rf(b.as_ptr(), inlen),
                );
            }
        }
    }
}

// ---------- rows 4 & 5 ----------
fn get_setters(lib: &Library) -> Vec<(&'static str, Box<dyn Fn(&mut [u32; 8], u64) + '_>)> {
    unsafe {
        let mut v: Vec<(&'static str, Box<dyn Fn(&mut [u32; 8], u64) + '_>)> = Vec::new();
        let f = sym!(lib, b"SPX_set_layer_addr\0", FAddrU32);
        v.push(("set_layer_addr", Box::new(move |a, x| f(a.as_mut_ptr(), x as u32))));
        let f = sym!(lib, b"SPX_set_tree_addr\0", FAddrU64);
        v.push(("set_tree_addr", Box::new(move |a, x| f(a.as_mut_ptr(), x))));
        let f = sym!(lib, b"SPX_set_type\0", FAddrU32);
        v.push(("set_type", Box::new(move |a, x| f(a.as_mut_ptr(), x as u32))));
        let f = sym!(lib, b"SPX_set_keypair_addr\0", FAddrU32);
        v.push(("set_keypair_addr", Box::new(move |a, x| f(a.as_mut_ptr(), x as u32))));
        let f = sym!(lib, b"SPX_set_chain_addr\0", FAddrU32);
        v.push(("set_chain_addr", Box::new(move |a, x| f(a.as_mut_ptr(), x as u32))));
        let f = sym!(lib, b"SPX_set_hash_addr\0", FAddrU32);
        v.push(("set_hash_addr", Box::new(move |a, x| f(a.as_mut_ptr(), x as u32))));
        let f = sym!(lib, b"SPX_set_tree_height\0", FAddrU32);
        v.push(("set_tree_height", Box::new(move |a, x| f(a.as_mut_ptr(), x as u32))));
        let f = sym!(lib, b"SPX_set_tree_index\0", FAddrU32);
        v.push(("set_tree_index", Box::new(move |a, x| f(a.as_mut_ptr(), x as u32))));
        v
    }
}

#[test]
fn b04_address_setters_random_sequences() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 4);
    let cs = get_setters(&p.c);
    let rs = get_setters(&p.r);
    assert_eq!(cs.len(), rs.len());

    // interesting values incl. out-of-byte-range ones
    let vals: [u64; 12] = [
        0,
        1,
        7,
        0xff,
        0x100,
        0x103,
        0xFFFF,
        0xFFFF_FFFF,
        0x1_0000_0000,
        u64::MAX,
        0x0102_0304_0506_0708,
        0x8000_0000,
    ];

    for trial in 0..300 {
        // start from either a zero address or a fully random one
        let start = if trial % 2 == 0 { [0u32; 8] } else { rng.addr() };
        let mut ca = start;
        let mut ra = start;
        let steps = 1 + rng.below(10);
        for _ in 0..steps {
            let k = rng.below(cs.len() as u32) as usize;
            let v = vals[rng.below(vals.len() as u32) as usize];
            (cs[k].1)(&mut ca, v);
            (rs[k].1)(&mut ra, v);
            eqb(
                &format!("addr after {} ({:#x})", cs[k].0, v),
                &addr_to_bytes(&ca),
                &addr_to_bytes(&ra),
            );
        }
    }
}

#[test]
fn b05_address_copies() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 5);
    unsafe {
        let csub = sym!(p.c, b"SPX_copy_subtree_addr\0", FAddrCopy);
        let rsub = sym!(p.r, b"SPX_copy_subtree_addr\0", FAddrCopy);
        let ckp = sym!(p.c, b"SPX_copy_keypair_addr\0", FAddrCopy);
        let rkp = sym!(p.r, b"SPX_copy_keypair_addr\0", FAddrCopy);
        for _ in 0..200 {
            let src = rng.addr();
            let dst = rng.addr();
            let mut c1 = dst;
            let mut r1 = dst;
            csub(c1.as_mut_ptr(), src.as_ptr());
            rsub(r1.as_mut_ptr(), src.as_ptr());
            eqb("copy_subtree_addr", &addr_to_bytes(&c1), &addr_to_bytes(&r1));

            let mut c2 = dst;
            let mut r2 = dst;
            ckp(c2.as_mut_ptr(), src.as_ptr());
            rkp(r2.as_mut_ptr(), src.as_ptr());
            eqb("copy_keypair_addr", &addr_to_bytes(&c2), &addr_to_bytes(&r2));
        }
        // zero source / zero dest edge cases
        let z = [0u32; 8];
        let ones = [0xFFFF_FFFFu32; 8];
        for (s, d) in [(z, ones), (ones, z)] {
            let mut c1 = d;
            let mut r1 = d;
            csub(c1.as_mut_ptr(), s.as_ptr());
            rsub(r1.as_mut_ptr(), s.as_ptr());
            eqb("copy_subtree_addr edge", &addr_to_bytes(&c1), &addr_to_bytes(&r1));
            let mut c2 = d;
            let mut r2 = d;
            ckp(c2.as_mut_ptr(), s.as_ptr());
            rkp(r2.as_mut_ptr(), s.as_ptr());
            eqb("copy_keypair_addr edge", &addr_to_bytes(&c2), &addr_to_bytes(&r2));
        }
    }
}

// ---------- row 6 ----------
#[test]
fn b06_initialize_hash_function() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 6);
    unsafe {
        for _ in 0..32 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            eqb("spx_ctx after initialize_hash_function", &cc, &rc);
        }
        // extremes
        for pat in [0x00u8, 0xff] {
            let ps = vec![pat; N];
            let ss = vec![pat ^ 0xff; N];
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            eqb("spx_ctx extremes", &cc, &rc);
        }
    }
}

// ---------- row 7 ----------
#[test]
fn b07_prf_addr() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 7);
    unsafe {
        let cf = sym!(p.c, b"SPX_prf_addr\0", FPrf);
        let rf = sym!(p.r, b"SPX_prf_addr\0", FPrf);
        for _ in 0..64 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            for _ in 0..8 {
                let a = rng.addr();
                let mut co = obuf(N);
                let mut ro = obuf(N);
                cf(co.as_mut_ptr(), cc.as_ptr(), a.as_ptr());
                rf(ro.as_mut_ptr(), rc.as_ptr(), a.as_ptr());
                eqb("prf_addr", &co, &ro);
            }
        }
    }
}

// ---------- rows 8-12 ----------
fn thash_case(p: &Pair, rng: &mut Rng, inblocks: u32, iters: usize) {
    unsafe {
        let cf = sym!(p.c, b"SPX_thash\0", FThash);
        let rf = sym!(p.r, b"SPX_thash\0", FThash);
        for _ in 0..iters {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            let inp = rng.bytes((inblocks as usize) * N);
            let a = rng.addr();
            let mut ca = a;
            let mut ra = a;
            let mut co = obuf(N);
            let mut ro = obuf(N);
            cf(co.as_mut_ptr(), inp.as_ptr(), inblocks, cc.as_ptr(), ca.as_mut_ptr());
            rf(ro.as_mut_ptr(), inp.as_ptr(), inblocks, rc.as_ptr(), ra.as_mut_ptr());
            eqb(&format!("thash out inblocks={inblocks}"), &co, &ro);
            eqb(
                &format!("thash addr inblocks={inblocks}"),
                &addr_to_bytes(&ca),
                &addr_to_bytes(&ra),
            );
        }
    }
}

#[test]
fn b08_thash_1() {
    let p = pair();
    p.check_config();
    thash_case(p, &mut Rng::new(SEED ^ 8), 1, 48);
}

#[test]
fn b09_thash_2() {
    let p = pair();
    thash_case(p, &mut Rng::new(SEED ^ 9), 2, 48);
}

#[test]
fn b10_thash_wots_len() {
    let p = pair();
    thash_case(p, &mut Rng::new(SEED ^ 10), WOTS_LEN as u32, 24);
}

#[test]
fn b11_thash_fors_trees() {
    let p = pair();
    thash_case(p, &mut Rng::new(SEED ^ 11), FORS_TREES as u32, 24);
}

#[test]
fn b12_thash_misc_widths() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 12);
    for ib in [0u32, 3, 4, 5, 7, 8, 16] {
        thash_case(p, &mut rng, ib, 8);
    }
}
