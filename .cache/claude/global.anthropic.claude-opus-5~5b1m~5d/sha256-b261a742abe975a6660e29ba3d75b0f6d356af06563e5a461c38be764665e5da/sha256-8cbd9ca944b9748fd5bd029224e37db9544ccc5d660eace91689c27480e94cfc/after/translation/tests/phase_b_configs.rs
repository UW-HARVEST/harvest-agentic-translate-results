//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both implementations are reached ONLY through `dlopen`/`dlsym` on their
//! respective `.so`s (see `common::c_merge_sort` / `common::rust_merge_sort`),
//! so the `#[no_mangle] extern "C"` wrapper is under test too.

mod common;

use common::*;

/// Number of guard elements appended after the logical end of each arena, so
/// that any write past `size - 1` shows up in the byte comparison.
const GUARD: usize = 3;

fn arena_for(sprites: &[Sprite], rng: &mut Rng) -> Arena {
    let mut ar = Arena::new((sprites.len() + GUARD) * ELEM_SIZE);
    ar.fill_random(rng); // random guard bytes + random `b` sentinel content
    ar.write_sprites(sprites);
    ar
}

/// `b` arena: same footprint, filled with a random sentinel, no sprites.
fn sentinel_arena(n: usize, rng: &mut Rng) -> Arena {
    let mut ar = Arena::new((n + GUARD) * ELEM_SIZE);
    ar.fill_random(rng);
    ar
}

fn sprite(sort_bits: i32, texture_id: u64) -> Sprite {
    Sprite { texture_id, sort_bits, _pad: 0 }
}

/// Random sprites; `key_alphabet == 0` means "full i32 range".
fn rand_sprites(n: usize, rng: &mut Rng, key_alphabet: u32) -> Vec<Sprite> {
    (0..n)
        .map(|_| {
            let sb = if key_alphabet == 0 {
                rng.next_i32()
            } else {
                (rng.next_u32() % key_alphabet) as i32
            };
            sprite(sb, rng.next_u64())
        })
        .collect()
}

/// Drive one configuration: fresh `a` from `sprites`, fresh sentinel `b`.
fn run_case(label: &str, sprites: &[Sprite], size: i32, rng: &mut Rng) {
    let a = arena_for(sprites, rng);
    let b = sentinel_arena(sprites.len(), rng);
    diff_run(label, &a, &b, size);
}

// ---------------------------------------------------------------------------
// Row 1 — size == 0
// ---------------------------------------------------------------------------
#[test]
fn row01_size_zero_no_mutation() {
    let mut rng = Rng::new(0x1001);
    for iter in 0..200 {
        let sprites = rand_sprites(0, &mut rng, 0);
        let a = arena_for(&sprites, &mut rng);
        let b = sentinel_arena(0, &mut rng);
        diff_run(&format!("row01/size=0/iter={iter}"), &a, &b, 0);

        // Additionally: neither buffer may be touched at all.
        let (mut ac, mut bc) = (a.clone(), b.clone());
        let before_a = ac.bytes().to_vec();
        let before_b = bc.bytes().to_vec();
        unsafe { c_merge_sort()(ac.ptr(), bc.ptr(), 0) };
        assert_eq!(ac.bytes(), &before_a[..], "row01: C mutated `a` for size=0");
        assert_eq!(bc.bytes(), &before_b[..], "row01: C mutated `b` for size=0");
        let (mut ar, mut br) = (a.clone(), b.clone());
        unsafe { rust_merge_sort()(ar.ptr(), br.ptr(), 0) };
        assert_eq!(ar.bytes(), &before_a[..], "row01: Rust mutated `a` for size=0");
        assert_eq!(br.bytes(), &before_b[..], "row01: Rust mutated `b` for size=0");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — size == 1
// ---------------------------------------------------------------------------
#[test]
fn row02_size_one() {
    let mut rng = Rng::new(0x1002);
    for iter in 0..500 {
        let sprites = rand_sprites(1, &mut rng, 0);
        run_case(&format!("row02/size=1/iter={iter}"), &sprites, 1, &mut rng);
    }
}

// ---------------------------------------------------------------------------
// Rows 3/4/5 — size == 2, ascending / descending / tied keys
// ---------------------------------------------------------------------------
#[test]
fn row03_size_two_ascending() {
    let mut rng = Rng::new(0x1003);
    for iter in 0..500 {
        let lo = rng.next_i32();
        let hi = lo.saturating_add(1 + (rng.next_u32() % 1000) as i32);
        let sprites = [sprite(lo, rng.next_u64()), sprite(hi, rng.next_u64())];
        run_case(&format!("row03/asc/iter={iter}"), &sprites, 2, &mut rng);
    }
}

#[test]
fn row04_size_two_descending() {
    let mut rng = Rng::new(0x1004);
    for iter in 0..500 {
        let lo = rng.next_i32();
        let hi = lo.saturating_add(1 + (rng.next_u32() % 1000) as i32);
        let sprites = [sprite(hi, rng.next_u64()), sprite(lo, rng.next_u64())];
        run_case(&format!("row04/desc/iter={iter}"), &sprites, 2, &mut rng);
    }
}

#[test]
fn row05_size_two_tied_keys_dead_branch() {
    let mut rng = Rng::new(0x1005);
    for iter in 0..500 {
        let sb = rng.next_i32();
        // distinct texture ids in both relative orders -> exercises the dead
        // second `if` of the comparison predicate.
        let (t0, t1) = (rng.next_u64(), rng.next_u64());
        for (x, y) in [(t0, t1), (t1, t0), (t0, t0)] {
            let sprites = [sprite(sb, x), sprite(sb, y)];
            run_case(&format!("row05/tied/iter={iter}/{x}-{y}"), &sprites, 2, &mut rng);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 6/7/8/9 — fixed small sizes
// ---------------------------------------------------------------------------
fn fixed_sizes(label: &str, seed: u64, sizes: &[usize], iters: usize) {
    let mut rng = Rng::new(seed);
    for &n in sizes {
        for iter in 0..iters {
            for alphabet in [0u32, 2, 4] {
                let sprites = rand_sprites(n, &mut rng, alphabet);
                run_case(
                    &format!("{label}/size={n}/alpha={alphabet}/iter={iter}"),
                    &sprites,
                    n as i32,
                    &mut rng,
                );
            }
        }
    }
}

#[test]
fn row06_size_three_odd_split() {
    fixed_sizes("row06", 0x1006, &[3], 200);
}

#[test]
fn row07_size_four_power_of_two() {
    fixed_sizes("row07", 0x1007, &[4], 200);
}

#[test]
fn row08_sizes_five_six_seven() {
    fixed_sizes("row08", 0x1008, &[5, 6, 7], 150);
}

#[test]
fn row09_powers_of_two() {
    fixed_sizes("row09", 0x1009, &[8, 16, 32, 64], 60);
}

// ---------------------------------------------------------------------------
// Row 10 — dense sweep, full-range random keys
// ---------------------------------------------------------------------------
#[test]
fn row10_dense_sweep_random_keys() {
    let mut rng = Rng::new(0x1010);
    for n in 1..=40usize {
        for iter in 0..40 {
            let sprites = rand_sprites(n, &mut rng, 0);
            run_case(&format!("row10/size={n}/iter={iter}"), &sprites, n as i32, &mut rng);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — tiny key alphabet (many ties)
// ---------------------------------------------------------------------------
#[test]
fn row11_dense_sweep_tiny_alphabet() {
    let mut rng = Rng::new(0x1011);
    for n in 1..=40usize {
        for iter in 0..40 {
            let sprites = rand_sprites(n, &mut rng, 3);
            run_case(&format!("row11/size={n}/iter={iter}"), &sprites, n as i32, &mut rng);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — all keys equal, texture_id varied (must be ignored identically)
// ---------------------------------------------------------------------------
#[test]
fn row12_all_keys_equal() {
    let mut rng = Rng::new(0x1012);
    for n in 1..=40usize {
        for mode in 0..3 {
            let sb = rng.next_i32();
            let sprites: Vec<Sprite> = (0..n)
                .map(|i| {
                    let t = match mode {
                        0 => i as u64,                 // strictly increasing
                        1 => (n - i) as u64,           // strictly decreasing
                        _ => rng.next_u64(),           // random
                    };
                    sprite(sb, t)
                })
                .collect();
            run_case(&format!("row12/size={n}/mode={mode}"), &sprites, n as i32, &mut rng);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — strictly ascending / strictly descending keys
// ---------------------------------------------------------------------------
#[test]
fn row13_sorted_and_reverse_sorted() {
    let mut rng = Rng::new(0x1013);
    for n in 1..=40usize {
        let asc: Vec<Sprite> = (0..n).map(|i| sprite(i as i32, rng.next_u64())).collect();
        run_case(&format!("row13/asc/size={n}"), &asc, n as i32, &mut rng);
        let desc: Vec<Sprite> =
            (0..n).map(|i| sprite((n - i) as i32, rng.next_u64())).collect();
        run_case(&format!("row13/desc/size={n}"), &desc, n as i32, &mut rng);
        // ascending with negative keys straddling zero
        let neg: Vec<Sprite> = (0..n)
            .map(|i| sprite(i as i32 - (n as i32) / 2, rng.next_u64()))
            .collect();
        run_case(&format!("row13/straddle/size={n}"), &neg, n as i32, &mut rng);
    }
}

// ---------------------------------------------------------------------------
// Row 14 — extreme key / texture_id values
// ---------------------------------------------------------------------------
#[test]
fn row14_extreme_values() {
    const KEYS: [i32; 7] = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    const TIDS: [u64; 4] = [0, 1, u64::MAX / 2, u64::MAX];
    let mut rng = Rng::new(0x1014);
    for n in 2..=24usize {
        for iter in 0..40 {
            let sprites: Vec<Sprite> = (0..n)
                .map(|_| {
                    sprite(KEYS[rng.below(KEYS.len())], TIDS[rng.below(TIDS.len())])
                })
                .collect();
            run_case(&format!("row14/size={n}/iter={iter}"), &sprites, n as i32, &mut rng);
        }
    }
    // Explicit INT_MIN / INT_MAX adjacency pairs, both orders.
    let mut rng = Rng::new(0x14FF);
    for &x in &KEYS {
        for &y in &KEYS {
            let sprites = [sprite(x, u64::MAX), sprite(y, 0)];
            run_case(&format!("row14/pair/{x}-{y}"), &sprites, 2, &mut rng);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — large inputs / deep recursion
// ---------------------------------------------------------------------------
#[test]
fn row15_large_inputs() {
    let mut rng = Rng::new(0x1015);
    for &n in &[1000usize, 4096, 4097] {
        for alphabet in [0u32, 5] {
            let sprites = rand_sprites(n, &mut rng, alphabet);
            run_case(&format!("row15/size={n}/alpha={alphabet}"), &sprites, n as i32, &mut rng);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 — sentinel `b` + guard elements (write-range fidelity)
// ---------------------------------------------------------------------------
#[test]
fn row16_guard_regions_and_sentinel() {
    let mut rng = Rng::new(0x1016);
    for n in 0..=33usize {
        for iter in 0..20 {
            // Big guard region so an overrun of several elements is visible.
            let mut a = Arena::new((n + 8) * ELEM_SIZE);
            a.fill_random(&mut rng);
            let sprites = rand_sprites(n, &mut rng, if iter % 2 == 0 { 0 } else { 3 });
            a.write_sprites(&sprites);
            let mut b = Arena::new((n + 8) * ELEM_SIZE);
            b.fill_random(&mut rng);
            diff_run(&format!("row16/size={n}/iter={iter}"), &a, &b, n as i32);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 17 — non-zero struct padding bytes
// ---------------------------------------------------------------------------
#[test]
fn row17_nonzero_padding() {
    let mut rng = Rng::new(0x1017);
    for n in 0..=17usize {
        for iter in 0..40 {
            // The arena starts as fully random bytes, so every element's 4
            // padding bytes are non-zero garbage; we then overwrite only the
            // two real fields, leaving padding untouched.
            let mut a = Arena::new((n + GUARD) * ELEM_SIZE);
            a.fill_random(&mut rng);
            for i in 0..n {
                let sb = if iter % 2 == 0 { rng.next_i32() } else { (rng.next_u32() % 3) as i32 };
                let tid = rng.next_u64();
                a.write_fields(i, tid, sb);
            }
            let mut b = Arena::new((n + GUARD) * ELEM_SIZE);
            b.fill_random(&mut rng);
            diff_run(&format!("row17/size={n}/iter={iter}"), &a, &b, n as i32);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18 — aliased buffers (a == b)
// ---------------------------------------------------------------------------
#[test]
fn row18_aliased_buffers() {
    let mut rng = Rng::new(0x1018);
    for n in 0..=17usize {
        for iter in 0..30 {
            let sprites = rand_sprites(n, &mut rng, if iter % 2 == 0 { 0 } else { 3 });
            let a = arena_for(&sprites, &mut rng);
            diff_run_aliased(&format!("row18/size={n}/iter={iter}"), &a, n as i32);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — `a` and `b` are windows into one arena with a varying gap
// ---------------------------------------------------------------------------
#[test]
fn row19_windows_varying_gap() {
    let mut rng = Rng::new(0x1019);
    for n in 1..=17usize {
        for iter in 0..30 {
            let gap = 1 + rng.below(4); // 1..=4 elements of separation
            let total = n + gap + n + GUARD;
            let mut arena = Arena::new(total * ELEM_SIZE);
            arena.fill_random(&mut rng);
            let sprites = rand_sprites(n, &mut rng, if iter % 2 == 0 { 0 } else { 3 });
            // write `a`'s sprites at offset 0
            arena.write_sprites(&sprites);
            let b_idx = n + gap;
            diff_run_windows(
                &format!("row19/size={n}/gap={gap}/iter={iter}"),
                &arena,
                0,
                b_idx,
                n as i32,
            );
            // and the mirrored layout: `b` before `a`
            diff_run_windows(
                &format!("row19-mirror/size={n}/gap={gap}/iter={iter}"),
                &arena,
                b_idx,
                0,
                n as i32,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — repeated invocation on the same buffers
// ---------------------------------------------------------------------------
#[test]
fn row20_repeated_invocation() {
    let mut rng = Rng::new(0x1020);
    for n in 1..=17usize {
        for iter in 0..30 {
            let sprites = rand_sprites(n, &mut rng, if iter % 2 == 0 { 0 } else { 3 });
            let a0 = arena_for(&sprites, &mut rng);
            let b0 = sentinel_arena(n, &mut rng);

            let (mut ac, mut bc) = (a0.clone(), b0.clone());
            let (mut ar, mut br) = (a0.clone(), b0.clone());
            for call in 0..3 {
                unsafe { c_merge_sort()(ac.ptr(), bc.ptr(), n as i32) };
                unsafe { rust_merge_sort()(ar.ptr(), br.ptr(), n as i32) };
                cmp_arena(&format!("row20/size={n}/iter={iter}/call={call}"), "a", &ac, &ar);
                cmp_arena(&format!("row20/size={n}/iter={iter}/call={call}"), "b", &bc, &br);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sanity: the two `.so`s really are two distinct files and both export the sym.
// ---------------------------------------------------------------------------
#[test]
fn harness_loads_two_distinct_libraries() {
    let c = c_so_path().clone();
    let r = rust_so_path().clone();
    assert_ne!(c, r, "harness must load two different shared objects");
    assert!(c.to_string_lossy().contains("c_src"), "unexpected C .so: {}", c.display());
    assert!(
        r.to_string_lossy().contains("merge_sort_lib"),
        "unexpected Rust .so: {}",
        r.display()
    );
    // Both symbols resolve.
    let _ = c_merge_sort();
    let _ = rust_merge_sort();
}
