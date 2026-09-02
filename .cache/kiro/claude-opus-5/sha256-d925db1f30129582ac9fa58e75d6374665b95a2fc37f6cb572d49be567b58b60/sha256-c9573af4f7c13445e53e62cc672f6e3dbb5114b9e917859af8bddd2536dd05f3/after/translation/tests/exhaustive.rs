// Exhaustive sweeps. These are the "no blind spot possible" tests: rather than
// sampling, they enumerate an entire input domain.
//
// `exhaustive_validate_and_normalize_full_i32` covers all 2^32 inputs and is
// therefore #[ignore]d by default; run it with
//   cargo test --release --test exhaustive -- --ignored --nocapture

mod common;
use common::*;

/// Every input in `lo..=hi`, compared one at a time.
fn sweep(lo: i32, hi: i32, tag: &str) {
    let p = LibPair::fresh(tag);
    let (c, r) = p.apis();
    let mut v = lo;
    loop {
        let cv = unsafe { (c.validate_and_normalize)(v) };
        let rv = unsafe { (r.validate_and_normalize)(v) };
        if cv != rv {
            panic!("{tag}: validate_and_normalize({v}): C={cv} Rust={rv}");
        }
        if v == hi {
            break;
        }
        v = v.wrapping_add(1);
    }
}

#[test]
fn exhaustive_validate_and_normalize_low_million() {
    // The entire interesting region, densely: covers every clamp boundary and
    // in particular value == 0o100 and value == 0o777, which is where a
    // `<` vs `<=` / `>` vs `>=` mutation would have to show up if it were an
    // observable difference at all.
    sweep(-(1 << 20), 1 << 20, "ex_low");
}

#[test]
fn exhaustive_validate_and_normalize_extremes() {
    sweep(i32::MIN, i32::MIN + (1 << 18), "ex_min");
    sweep(i32::MAX - (1 << 18), i32::MAX, "ex_max");
}

#[test]
#[ignore = "4.3 billion FFI calls; run explicitly with --ignored"]
fn exhaustive_validate_and_normalize_full_i32() {
    let p = LibPair::fresh("ex_full");
    let (c, r) = p.apis();
    let mut v: i32 = i32::MIN;
    let mut n: u64 = 0;
    loop {
        let cv = unsafe { (c.validate_and_normalize)(v) };
        let rv = unsafe { (r.validate_and_normalize)(v) };
        if cv != rv {
            panic!("validate_and_normalize({v}): C={cv} Rust={rv}");
        }
        n += 1;
        if v == i32::MAX {
            break;
        }
        v = v.wrapping_add(1);
    }
    assert_eq!(n, 1u64 << 32, "must have covered every i32");
    eprintln!("exhaustive: {n} inputs agreed");
}

/// Every `(string byte, needle low byte)` pair for `find_and_replace_char`:
/// 256 x 256 = 65536 combinations, each in three positions.
#[test]
fn exhaustive_find_and_replace_byte_matrix() {
    let p = LibPair::fresh("ex_bytes");
    let (c, r) = p.apis();
    for sb in 1u32..256 {
        // byte 0 cannot appear inside a C string
        for nb in 0u32..256 {
            // three shapes: needle byte first, middle, last
            let strings: [Vec<u8>; 3] = [
                vec![sb as u8, b'.', b'.'],
                vec![b'.', sb as u8, b'.'],
                vec![b'.', b'.', sb as u8],
            ];
            for s in &strings {
                // present the needle as several distinct ints sharing nb
                for needle in [nb as i32, (nb as i32) | 0x100, (nb as i32).wrapping_sub(256)] {
                    let mut cb = scratch(0xAA);
                    let mut rb = scratch(0xAA);
                    set_cstr(&mut cb, s);
                    set_cstr(&mut rb, s);
                    unsafe { (c.find_and_replace_char)(cb.as_mut_ptr(), needle) };
                    unsafe { (r.find_and_replace_char)(rb.as_mut_ptr(), needle) };
                    assert_eq!(
                        as_u8(&cb),
                        as_u8(&rb),
                        "byte matrix sb=0x{sb:02x} needle={needle}:\n  C   ={}\n  Rust={}",
                        show(&cb),
                        show(&rb)
                    );
                }
            }
        }
    }
}

/// Every `process_octal_string` value in a dense band plus both extremes,
/// comparing the full 256-byte destination buffer.
#[test]
fn exhaustive_process_octal_string_dense_band() {
    let p = LibPair::fresh("ex_octal");
    let (c, r) = p.apis();
    let check = |v: i32| {
        let mut cb = scratch(0xAA);
        let mut rb = scratch(0xAA);
        unsafe { (c.process_octal_string)(cb.as_mut_ptr(), v) };
        unsafe { (r.process_octal_string)(rb.as_mut_ptr(), v) };
        assert_eq!(
            as_u8(&cb),
            as_u8(&rb),
            "process_octal_string({v}):\n  C   ={}\n  Rust={}",
            show(&cb),
            show(&rb)
        );
    };
    // dense: every value in -100000..=100000 (crosses every octal digit-count
    // boundary in that range, and both signs)
    for v in -100_000i32..=100_000 {
        check(v);
    }
    // every octal digit-count boundary across the whole range, both signs
    for k in 0..32u32 {
        let base = 1i32.checked_shl(k).unwrap_or(i32::MIN);
        for d in -2i32..=2 {
            check(base.wrapping_add(d));
            check(base.wrapping_neg().wrapping_add(d));
        }
    }
    check(i32::MIN);
    check(i32::MIN + 1);
    check(i32::MAX);
    check(i32::MAX - 1);
}

/// The full 2^16 space of `findrep`'s dispatch-relevant small-parameter grid:
/// each parameter drawn from a 8-value set that hits every normalization
/// bucket, cross-producted (8^4 = 4096) and re-run on a fresh state in batches.
#[test]
fn exhaustive_findrep_small_grid() {
    const V: [i32; 8] = [0, 1, 63, 64, 65, 511, 512, -1];
    const BATCH: usize = 32;
    let mut n = 0usize;
    let mut pair = LibPair::fresh("ex_grid_0");
    let mut apis = pair.apis();
    for &a in &V {
        for &b in &V {
            for &c3 in &V {
                for &d in &V {
                    if n % BATCH == 0 && n != 0 {
                        drop(apis);
                        pair = LibPair::fresh(&format!("ex_grid_{n}"));
                        apis = pair.apis();
                    }
                    let cv = unsafe { (apis.0.findrep)(a, b, c3, d) };
                    let rv = unsafe { (apis.1.findrep)(a, b, c3, d) };
                    assert_eq!(cv, rv, "#{n} findrep({a},{b},{c3},{d}) C={cv} Rust={rv}");
                    n += 1;
                }
            }
        }
    }
    assert_eq!(n, 4096);
}
