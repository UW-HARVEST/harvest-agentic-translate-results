//! Phase B — differential tests for the lowest-level entry points:
//! `app/src/utils.c` (byte conversions) and `app/src/address.c`.

mod common;
use common::*;
use std::ffi::c_void;

type UllToBytes = unsafe extern "C" fn(*mut u8, u32, u64);
type U32ToBytes = unsafe extern "C" fn(*mut u8, u32);
type BytesToUll = unsafe extern "C" fn(*const u8, u32) -> u64;
type Set1 = unsafe extern "C" fn(*mut u32, u32);
type SetTree = unsafe extern "C" fn(*mut u32, u64);
type Copy2 = unsafe extern "C" fn(*mut u32, *const u32);

#[test]
fn harness_loads_and_test_exe_does_not_export_library_symbols() {
    let libs = Libs::load();
    // Sanity: the two implementations of a trivial function are distinct
    // addresses, i.e. we really loaded two libraries.
    let (c, r) = libs.pair::<U32ToBytes>("SPX_u32_to_bytes");
    assert_ne!(*c as usize, *r as usize, "C and Rust symbols resolved to the same address — the test executable is interposing");
}

#[test]
fn ull_to_bytes_all_outlens() {
    let libs = Libs::load();
    let (c, r) = libs.pair::<UllToBytes>("SPX_ull_to_bytes");
    let mut rng = Rng::new(0x1111);
    for outlen in 0u32..=16 {
        for _ in 0..200 {
            let v = rng.next_u64();
            let mut cb = vec![0xAAu8; 32];
            let mut rb = vec![0xAAu8; 32];
            unsafe {
                c(cb.as_mut_ptr(), outlen, v);
                r(rb.as_mut_ptr(), outlen, v);
            }
            assert_bytes_eq(&format!("ull_to_bytes(outlen={}, in={:#x})", outlen, v), &cb, &rb);
        }
        // boundary values
        for v in [0u64, 1, 0xff, 0x100, u64::MAX, u64::MAX - 1, 1 << 63] {
            let mut cb = vec![0x55u8; 32];
            let mut rb = vec![0x55u8; 32];
            unsafe {
                c(cb.as_mut_ptr(), outlen, v);
                r(rb.as_mut_ptr(), outlen, v);
            }
            assert_bytes_eq(&format!("ull_to_bytes(outlen={}, in={:#x})", outlen, v), &cb, &rb);
        }
    }
}

#[test]
fn u32_to_bytes_random() {
    let libs = Libs::load();
    let (c, r) = libs.pair::<U32ToBytes>("SPX_u32_to_bytes");
    let mut rng = Rng::new(0x2222);
    let mut vals: Vec<u32> = vec![0, 1, 0xff, 0x100, 0xffff, 0xffff_ffff, 0x8000_0000];
    for _ in 0..2000 {
        vals.push(rng.next_u32());
    }
    for v in vals {
        let mut cb = [0xAAu8; 8];
        let mut rb = [0xAAu8; 8];
        unsafe {
            c(cb.as_mut_ptr(), v);
            r(rb.as_mut_ptr(), v);
        }
        assert_bytes_eq(&format!("u32_to_bytes({:#x})", v), &cb, &rb);
    }
}

#[test]
fn bytes_to_ull_all_inlens() {
    let libs = Libs::load();
    let (c, r) = libs.pair::<BytesToUll>("SPX_bytes_to_ull");
    let mut rng = Rng::new(0x3333);
    // inlen 0..=8 is the documented range (any more would shift a u64 by >= 64,
    // which is undefined behaviour in C).
    for inlen in 0u32..=8 {
        for _ in 0..300 {
            let b = rng.bytes(16);
            let cv = unsafe { c(b.as_ptr(), inlen) };
            let rv = unsafe { r(b.as_ptr(), inlen) };
            assert_eq!(cv, rv, "bytes_to_ull(inlen={}, {})", inlen, hex(&b));
        }
        for pat in [0x00u8, 0xff, 0x80, 0x01] {
            let b = vec![pat; 16];
            let cv = unsafe { c(b.as_ptr(), inlen) };
            let rv = unsafe { r(b.as_ptr(), inlen) };
            assert_eq!(cv, rv, "bytes_to_ull(inlen={}, pat={:#x})", inlen, pat);
        }
    }
}

fn addr_bytes(a: &[u32; 8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, w) in a.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&w.to_ne_bytes());
    }
    out
}

#[test]
fn address_setters_random_and_boundary() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x4444);

    let one_arg: [&str; 7] = [
        "SPX_set_layer_addr",
        "SPX_set_type",
        "SPX_set_keypair_addr",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
        "SPX_set_tree_index",
    ];

    for name in one_arg {
        let (c, r) = libs.pair::<Set1>(name);
        let mut vals: Vec<u32> = vec![
            0, 1, 2, 3, 4, 5, 6, 7, 8, 0xff, 0x100, 0x101, 0xffff, 0xffff_ff00, 0xffff_ffff,
            0x8000_0000, 0x7fff_ffff,
        ];
        for _ in 0..500 {
            vals.push(rng.next_u32());
        }
        for v in vals {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            assert_bytes_eq(
                &format!("{}({:#x})", name, v),
                &addr_bytes(&ca),
                &addr_bytes(&ra),
            );
        }
    }

    // set_tree_addr takes a uint64_t
    let (c, r) = libs.pair::<SetTree>("SPX_set_tree_addr");
    let mut vals: Vec<u64> = vec![0, 1, 0xff, 0x100, u64::MAX, 1 << 63, (1u64 << 40) - 1];
    for _ in 0..500 {
        vals.push(rng.next_u64());
    }
    for v in vals {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        assert_bytes_eq(
            &format!("SPX_set_tree_addr({:#x})", v),
            &addr_bytes(&ca),
            &addr_bytes(&ra),
        );
    }

    // the two copy helpers
    for name in ["SPX_copy_subtree_addr", "SPX_copy_keypair_addr"] {
        let (c, r) = libs.pair::<Copy2>(name);
        for _ in 0..500 {
            let src = rng.addr();
            let dst = rng.addr();
            let mut ca = dst;
            let mut ra = dst;
            unsafe {
                c(ca.as_mut_ptr(), src.as_ptr());
                r(ra.as_mut_ptr(), src.as_ptr());
            }
            assert_bytes_eq(name, &addr_bytes(&ca), &addr_bytes(&ra));
        }
    }
}

/// The exact sequence of address mutations the signer performs, applied to the
/// same buffer in both libraries (catches an offset table mismatch that a
/// single setter call could mask).
#[test]
fn address_setter_sequences() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x5555);

    let layer = libs.pair::<Set1>("SPX_set_layer_addr");
    let ttype = libs.pair::<Set1>("SPX_set_type");
    let kp = libs.pair::<Set1>("SPX_set_keypair_addr");
    let chain = libs.pair::<Set1>("SPX_set_chain_addr");
    let hash = libs.pair::<Set1>("SPX_set_hash_addr");
    let hgt = libs.pair::<Set1>("SPX_set_tree_height");
    let tidx = libs.pair::<Set1>("SPX_set_tree_index");
    let tree = libs.pair::<SetTree>("SPX_set_tree_addr");
    let cpsub = libs.pair::<Copy2>("SPX_copy_subtree_addr");
    let cpkp = libs.pair::<Copy2>("SPX_copy_keypair_addr");

    for _ in 0..300 {
        let mut ca = [0u32; 8];
        let mut ra = [0u32; 8];
        let mut cb = [0u32; 8];
        let mut rb = [0u32; 8];
        let l = rng.below(D as u32);
        let t = rng.next_u64();
        let k = rng.next_u32();
        let ch = rng.next_u32();
        let h = rng.next_u32();
        let g = rng.below(TREE_HEIGHT as u32 + 1);
        let ti = rng.next_u32();
        let ty = rng.below(7);
        unsafe {
            for (i, (f, _)) in [(&layer, 0)].iter().enumerate() {
                let _ = i;
                (f.0)(ca.as_mut_ptr(), l);
                (f.1)(ra.as_mut_ptr(), l);
            }
            (tree.0)(ca.as_mut_ptr(), t);
            (tree.1)(ra.as_mut_ptr(), t);
            (ttype.0)(ca.as_mut_ptr(), ty);
            (ttype.1)(ra.as_mut_ptr(), ty);
            (kp.0)(ca.as_mut_ptr(), k);
            (kp.1)(ra.as_mut_ptr(), k);
            (cpsub.0)(cb.as_mut_ptr(), ca.as_ptr());
            (cpsub.1)(rb.as_mut_ptr(), ra.as_ptr());
            (cpkp.0)(cb.as_mut_ptr(), ca.as_ptr());
            (cpkp.1)(rb.as_mut_ptr(), ra.as_ptr());
            (chain.0)(ca.as_mut_ptr(), ch);
            (chain.1)(ra.as_mut_ptr(), ch);
            (hash.0)(ca.as_mut_ptr(), h);
            (hash.1)(ra.as_mut_ptr(), h);
            (hgt.0)(cb.as_mut_ptr(), g);
            (hgt.1)(rb.as_mut_ptr(), g);
            (tidx.0)(cb.as_mut_ptr(), ti);
            (tidx.1)(rb.as_mut_ptr(), ti);
        }
        assert_bytes_eq("sequence/addr", &addr_bytes(&ca), &addr_bytes(&ra));
        assert_bytes_eq("sequence/copy", &addr_bytes(&cb), &addr_bytes(&rb));
    }
}

#[test]
fn initialize_hash_function_matches() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x6666);
    for _ in 0..50 {
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);
        assert_bytes_eq("spx_ctx after initialize_hash_function", cc.live(), rc.live());
    }
    // extreme seeds
    for (ps, ss) in [
        (vec![0u8; N], vec![0u8; N]),
        (vec![0xffu8; N], vec![0xffu8; N]),
        (vec![0u8; N], vec![0xffu8; N]),
    ] {
        let (cc, rc) = init_ctx_pair(&libs, &ps, &ss);
        assert_bytes_eq("spx_ctx after initialize_hash_function", cc.live(), rc.live());
    }
    let _ = std::mem::size_of::<*const c_void>();
}
