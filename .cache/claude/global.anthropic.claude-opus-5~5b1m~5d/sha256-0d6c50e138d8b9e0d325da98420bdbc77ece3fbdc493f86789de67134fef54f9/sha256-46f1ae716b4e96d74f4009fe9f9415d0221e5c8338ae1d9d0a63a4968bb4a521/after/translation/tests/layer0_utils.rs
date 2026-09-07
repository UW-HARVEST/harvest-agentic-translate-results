//! Phase B rows 1-10: the lowest-level entry points (`utils.c`, `address.c`).
//!
//! Every call goes through `dlopen`/`dlsym` on BOTH `.so`s.
mod common;
use common::*;

type FnUllToBytes = unsafe extern "C" fn(*mut u8, core::ffi::c_uint, core::ffi::c_ulonglong);
type FnU32ToBytes = unsafe extern "C" fn(*mut u8, u32);
type FnBytesToUll =
    unsafe extern "C" fn(*const u8, core::ffi::c_uint) -> core::ffi::c_ulonglong;
type FnAddrU32 = unsafe extern "C" fn(*mut u32, u32);
type FnAddrU64 = unsafe extern "C" fn(*mut u32, u64);
type FnAddrCopy = unsafe extern "C" fn(*mut u32, *const u32);

const ITERS: usize = 256;

// --- row 1 / row 2 -------------------------------------------------------

fn ull_to_bytes_case(outlen: usize, values: &[u64]) {
    let l = libs();
    let (c, r) = l.pair::<FnUllToBytes>("SPX_ull_to_bytes");
    for &v in values {
        // Pre-fill with a recognisable pattern so we also detect writes the C
        // does NOT perform.
        let mut cb = vec![0xA5u8; outlen + 8];
        let mut rb = vec![0xA5u8; outlen + 8];
        unsafe {
            c(cb.as_mut_ptr(), outlen as core::ffi::c_uint, v);
            r(rb.as_mut_ptr(), outlen as core::ffi::c_uint, v);
        }
        assert_bytes_eq(&format!("SPX_ull_to_bytes(outlen={outlen}, in={v:#x})"), &cb, &rb);
    }
}

#[test]
fn row01_ull_to_bytes_small_outlen() {
    let mut rng = Rng::new(0x0101);
    let mut values = vec![0u64, 1, 2, 0xFF, 0x100, u64::MAX, u64::MAX - 1, 0x8000_0000_0000_0000];
    for _ in 0..ITERS {
        values.push(rng.next_u64());
    }
    for outlen in [0usize, 1, 2, 3, 4, 5, 6, 7, 8] {
        ull_to_bytes_case(outlen, &values);
    }
}

#[test]
fn row02_ull_to_bytes_outlen_gt_8() {
    let mut rng = Rng::new(0x0202);
    let mut values = vec![0u64, 1, u64::MAX];
    for _ in 0..ITERS {
        values.push(rng.next_u64());
    }
    for outlen in [9usize, 10, 12, 16, 32] {
        ull_to_bytes_case(outlen, &values);
    }
}

// --- row 3 --------------------------------------------------------------

#[test]
fn row03_u32_to_bytes() {
    let l = libs();
    let (c, r) = l.pair::<FnU32ToBytes>("SPX_u32_to_bytes");
    let mut rng = Rng::new(0x0303);
    let mut values = vec![0u32, 1, 0xFF, 0x100, 0xFFFF, u32::MAX, 0x8000_0000];
    for _ in 0..ITERS {
        values.push(rng.next_u32());
    }
    for v in values {
        let mut cb = [0xA5u8; 12];
        let mut rb = [0xA5u8; 12];
        unsafe {
            c(cb.as_mut_ptr(), v);
            r(rb.as_mut_ptr(), v);
        }
        assert_bytes_eq(&format!("SPX_u32_to_bytes({v:#x})"), &cb, &rb);
    }
}

// --- row 4 / row 5 -------------------------------------------------------

fn bytes_to_ull_case(inlen: usize, inputs: &[Vec<u8>]) {
    let l = libs();
    let (c, r) = l.pair::<FnBytesToUll>("SPX_bytes_to_ull");
    for inp in inputs {
        let cv = unsafe { c(inp.as_ptr(), inlen as core::ffi::c_uint) };
        let rv = unsafe { r(inp.as_ptr(), inlen as core::ffi::c_uint) };
        assert_eq_dbg(
            &format!("SPX_bytes_to_ull(inlen={inlen}, in={})", hex(&inp[..inlen.min(inp.len())])),
            cv,
            rv,
        );
    }
}

#[test]
fn row04_bytes_to_ull_small_inlen() {
    let mut rng = Rng::new(0x0404);
    let mut inputs: Vec<Vec<u8>> = vec![vec![0u8; 40], vec![0xFFu8; 40], vec![0x80u8; 40]];
    for _ in 0..ITERS {
        inputs.push(rng.bytes(40));
    }
    for inlen in [0usize, 1, 2, 3, 4, 5, 6, 7, 8] {
        bytes_to_ull_case(inlen, &inputs);
    }
}

#[test]
fn row05_bytes_to_ull_inlen_gt_8() {
    // inlen > 8 makes the C shift by >= 64 bits (UB in C, but a real input the
    // C accepts; clang -O3 on x86-64 produces a specific value the Rust must
    // reproduce).
    let mut rng = Rng::new(0x0505);
    let mut inputs: Vec<Vec<u8>> = vec![vec![0u8; 40], vec![0xFFu8; 40], vec![0x01u8; 40]];
    for _ in 0..ITERS {
        inputs.push(rng.bytes(40));
    }
    for inlen in [9usize, 10, 12, 16] {
        bytes_to_ull_case(inlen, &inputs);
    }
}

// --- row 6 / row 7 / row 8 -----------------------------------------------

#[test]
fn row06_addr_single_byte_setters() {
    let l = libs();
    let names = [
        "SPX_set_layer_addr",
        "SPX_set_type",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
    ];
    let mut rng = Rng::new(0x0606);
    for name in names {
        let (c, r) = l.pair::<FnAddrU32>(name);
        for i in 0..ITERS {
            let base = rng.addr();
            // Cover the documented range, byte boundaries, and out-of-range.
            let v = match i {
                0..=6 => i as u32,
                7 => 7,
                8 => 255,
                9 => 256,
                10 => u32::MAX,
                _ => rng.next_u32(),
            };
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            assert_bytes_eq(
                &format!("{name}({v:#x}) on {}", hex(&addr_bytes(&base))),
                &addr_bytes(&ca),
                &addr_bytes(&ra),
            );
        }
    }
}

#[test]
fn row07_set_tree_addr() {
    let l = libs();
    let (c, r) = l.pair::<FnAddrU64>("SPX_set_tree_addr");
    let mut rng = Rng::new(0x0707);
    let mut values = vec![0u64, 1, u64::MAX, 1u64 << 63, (1u64 << SPX_TREE_BITS.min(63)) - 1];
    for _ in 0..ITERS {
        values.push(rng.next_u64());
    }
    for v in values {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        assert_bytes_eq(
            &format!("SPX_set_tree_addr({v:#x})"),
            &addr_bytes(&ca),
            &addr_bytes(&ra),
        );
    }
}

#[test]
fn row08_addr_u32_field_setters() {
    let l = libs();
    let mut rng = Rng::new(0x0808);
    for name in ["SPX_set_keypair_addr", "SPX_set_tree_index"] {
        let (c, r) = l.pair::<FnAddrU32>(name);
        let mut values = vec![0u32, 1, 0xFF, 0x100, 0xFFFF, 0xFF_FFFF, u32::MAX];
        for _ in 0..ITERS {
            values.push(rng.next_u32());
        }
        for v in values {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            assert_bytes_eq(
                &format!("{name}({v:#x})"),
                &addr_bytes(&ca),
                &addr_bytes(&ra),
            );
        }
    }
}

// --- row 9 --------------------------------------------------------------

#[test]
fn row09_addr_copy_setters() {
    let l = libs();
    let mut rng = Rng::new(0x0909);
    for name in ["SPX_copy_subtree_addr", "SPX_copy_keypair_addr"] {
        let (c, r) = l.pair::<FnAddrCopy>(name);
        for _ in 0..ITERS {
            let src = rng.addr();
            let dst = rng.addr();
            let mut ca = dst;
            let mut ra = dst;
            unsafe {
                c(ca.as_mut_ptr(), src.as_ptr());
                r(ra.as_mut_ptr(), src.as_ptr());
            }
            assert_bytes_eq(
                &format!("{name} src={} dst={}", hex(&addr_bytes(&src)), hex(&addr_bytes(&dst))),
                &addr_bytes(&ca),
                &addr_bytes(&ra),
            );
        }
    }
}

// --- row 10 -------------------------------------------------------------

#[test]
fn row10_addr_setter_composition() {
    // Apply every setter in the order sign.c / wotsx1.c / fors.c use them, on
    // one address, so aliasing between CHAIN_ADDR/TREE_HGT (both offset 27 on
    // haraka/shake/blake, 17 on sha2) and HASH_ADDR/TREE_INDEX is exercised.
    let l = libs();
    let (c_layer, r_layer) = l.pair::<FnAddrU32>("SPX_set_layer_addr");
    let (c_tree, r_tree) = l.pair::<FnAddrU64>("SPX_set_tree_addr");
    let (c_type, r_type) = l.pair::<FnAddrU32>("SPX_set_type");
    let (c_kp, r_kp) = l.pair::<FnAddrU32>("SPX_set_keypair_addr");
    let (c_chain, r_chain) = l.pair::<FnAddrU32>("SPX_set_chain_addr");
    let (c_hash, r_hash) = l.pair::<FnAddrU32>("SPX_set_hash_addr");
    let (c_hgt, r_hgt) = l.pair::<FnAddrU32>("SPX_set_tree_height");
    let (c_idx, r_idx) = l.pair::<FnAddrU32>("SPX_set_tree_index");
    let (c_sub, r_sub) = l.pair::<FnAddrCopy>("SPX_copy_subtree_addr");
    let (c_ckp, r_ckp) = l.pair::<FnAddrCopy>("SPX_copy_keypair_addr");

    let mut rng = Rng::new(0x1010);
    for _ in 0..ITERS {
        let base = rng.addr();
        let other = rng.addr();
        let (layer, tree, ty, kp, chain, hash, hgt, idx) = (
            rng.next_u32(),
            rng.next_u64(),
            rng.next_u32() % 7,
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
            rng.next_u32(),
        );
        let mut ca = base;
        let mut ra = base;
        unsafe {
            for (a, fl, ft, fy, fk, fc, fh, fg, fi, fs, fck) in [
                (
                    &mut ca as *mut [u32; 8],
                    &c_layer, &c_tree, &c_type, &c_kp, &c_chain, &c_hash, &c_hgt, &c_idx, &c_sub, &c_ckp,
                ),
                (
                    &mut ra as *mut [u32; 8],
                    &r_layer, &r_tree, &r_type, &r_kp, &r_chain, &r_hash, &r_hgt, &r_idx, &r_sub, &r_ckp,
                ),
            ] {
                let p = (*a).as_mut_ptr();
                fl(p, layer);
                ft(p, tree);
                fy(p, ty);
                fk(p, kp);
                fc(p, chain);
                fh(p, hash);
                fg(p, hgt);
                fi(p, idx);
                fs(p, other.as_ptr());
                fck(p, other.as_ptr());
            }
        }
        assert_bytes_eq("address setter composition", &addr_bytes(&ca), &addr_bytes(&ra));
    }
}
