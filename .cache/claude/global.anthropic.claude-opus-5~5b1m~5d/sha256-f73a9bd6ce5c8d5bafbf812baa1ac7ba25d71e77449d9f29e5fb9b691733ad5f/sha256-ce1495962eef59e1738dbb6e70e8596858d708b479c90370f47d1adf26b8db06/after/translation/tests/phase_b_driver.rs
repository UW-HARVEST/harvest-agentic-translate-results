//! Phase B — CONFIGS.md rows 47-50: the `strkey` helper and the `hm_geti`
//! driver (the only symbol declared in `include/lib.h`).

mod common;

use common::*;
use std::ffi::c_void;

/// The whole 256-byte static buffer, so residue left by previous (longer)
/// `sprintf` calls is compared too.
unsafe fn strkey_buf(api: &Api, n: i32) -> (Vec<u8>, Vec<u8>) {
    unsafe {
        let p = (api.strkey)(n);
        let cs = std::ffi::CStr::from_ptr(p).to_bytes().to_vec();
        let all = std::slice::from_raw_parts(p as *const u8, 256).to_vec();
        (cs, all)
    }
}

/// row 47 — `strkey` value sweep
#[test]
fn strkey_values() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    let mut values: Vec<i32> = vec![
        0,
        1,
        -1,
        9,
        10,
        11,
        99,
        100,
        42,
        99999,
        1_000_000_000,
        i32::MAX,
        i32::MIN,
        i32::MIN + 1,
        -2147483647,
    ];
    let mut rng = Rng::new(0x57_0000);
    for _ in 0..200 {
        values.push(rng.next_u64() as i32);
    }
    // interleave long and short so the static buffer keeps residue
    values.push(i32::MIN);
    values.push(0);
    values.push(i32::MAX);
    values.push(-1);

    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        for &v in &values {
            unsafe {
                let (cs, all) = strkey_buf(api, v);
                t.push(format!("strkey({v}) = {:?}", String::from_utf8_lossy(&cs)));
                t.push(format!("  buf={}", hex(&all)));
            }
        }
    }
    assert_traces_eq("strkey values", &tc, &tr);
}

/// Observe the global hash seed by creating a fresh table and reading
/// `table->seed`; also snapshots that table.
unsafe fn observe_seed(api: &Api) -> Vec<String> {
    unsafe {
        let h = (api.shmode_func)(8, SH_NONE);
        let t = map_table(h, 8);
        let mut out = vec![format!("observed table.seed={:#x}", (*t).seed)];
        out.extend(snap_map(h, 8, KeyKind::Binary));
        (api.hmfree_func)(map_raw(h, 8) as *mut c_void, 8);
        out
    }
}

/// row 48 — `hm_geti(num)` across the counts that exercise its internal grow /
/// delete / shrink / rebuild paths.  The function is self-checking: a behaviour
/// difference trips one of its `assert`s and aborts.  Afterwards the global
/// seed advance is compared.
#[test]
fn hm_geti_counts() {
    for num in [0i32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 16, 17, 24, 32, 50, 100, 400, 1000] {
        let p = seeded(DEFAULT_SEED);
        let mut tc = Vec::new();
        let mut tr = Vec::new();
        for api in p.both() {
            let t = if api.tag == "C" { &mut tc } else { &mut tr };
            unsafe {
                (api.rand_seed)(DEFAULT_SEED);
                (api.hm_geti)(num);
                t.push(format!("hm_geti({num}) returned"));
                t.extend(observe_seed(api));
            }
        }
        assert_traces_eq(&format!("hm_geti({num})"), &tc, &tr);
    }
}

/// row 49 — `num <= 0`: every loop is skipped
#[test]
fn hm_geti_nonpositive() {
    for num in [0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        let p = seeded(DEFAULT_SEED);
        let mut tc = Vec::new();
        let mut tr = Vec::new();
        for api in p.both() {
            let t = if api.tag == "C" { &mut tc } else { &mut tr };
            unsafe {
                (api.rand_seed)(DEFAULT_SEED);
                (api.hm_geti)(num);
                t.push(format!("hm_geti({num}) returned"));
                t.extend(observe_seed(api));
            }
        }
        assert_traces_eq(&format!("hm_geti({num}) nonpositive"), &tc, &tr);
    }
}

/// row 50 — `hm_geti` under different global seeds (different probe orders)
#[test]
fn hm_geti_seeds() {
    let mut rng = Rng::new(0x5A_0000);
    let mut starts: Vec<usize> = vec![0, 1, 2, DEFAULT_SEED, usize::MAX, usize::MAX - 1];
    for _ in 0..8 {
        starts.push(rng.next_u64() as usize);
    }
    for s in starts {
        for num in [1i32, 5, 17, 64, 200] {
            let p = seeded(s);
            let mut tc = Vec::new();
            let mut tr = Vec::new();
            for api in p.both() {
                let t = if api.tag == "C" { &mut tc } else { &mut tr };
                unsafe {
                    (api.rand_seed)(s);
                    (api.hm_geti)(num);
                    t.push(format!("hm_geti({num}) seed={s:#x} returned"));
                    t.extend(observe_seed(api));
                }
            }
            assert_traces_eq(&format!("hm_geti({num}) seed={s:#x}"), &tc, &tr);
        }
    }
}

/// Extra: `hm_geti` called repeatedly in one process (the seed keeps advancing)
#[test]
fn hm_geti_repeated() {
    let p = seeded(DEFAULT_SEED);
    let mut tc = Vec::new();
    let mut tr = Vec::new();
    for api in p.both() {
        let t = if api.tag == "C" { &mut tc } else { &mut tr };
        unsafe {
            (api.rand_seed)(DEFAULT_SEED);
            for round in 0..10 {
                (api.hm_geti)(3 + round * 7);
                t.push(format!("round {round} ok"));
                t.extend(observe_seed(api));
            }
        }
    }
    assert_traces_eq("hm_geti repeated", &tc, &tr);
}
