//! Level 4: string-keyed maps – `stbds_shmode_func` plus `stbds_hmput_key` /
//! `stbds_hmget_key` / `stbds_hmdel_key` driven in `STBDS_HM_STRING` mode for
//! every `string.mode` (none / default / strdup / arena).

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

/// `sizeof (t)->key` for a `char *` key.
const KEYSIZE: usize = 8;

/// Keeps every key buffer alive for the whole script: in `STBDS_SH_DEFAULT`
/// mode stb_ds stores the caller's pointer verbatim.
struct Keys {
    bufs: Vec<Box<Vec<u8>>>,
}

impl Keys {
    fn new() -> Keys {
        Keys { bufs: Vec::new() }
    }
    fn make(&mut self, s: &str) -> *mut c_char {
        let mut v = Box::new(s.as_bytes().to_vec());
        v.push(0);
        let p = v.as_mut_ptr() as *mut c_char;
        self.bufs.push(v);
        p
    }
}

unsafe fn shput(lib: &Lib, t: *mut u8, elemsize: usize, k: *mut c_char, value: &[u8]) -> *mut u8 {
    let t = (lib.hmput_key)(
        t as *mut c_void,
        elemsize,
        k as *mut c_void,
        KEYSIZE,
        HM_STRING,
    ) as *mut u8;
    let idx = temp_of(t, elemsize);
    let e = t.offset(idx * elemsize as isize);
    std::ptr::copy_nonoverlapping(value.as_ptr(), e.add(KEYSIZE), value.len());
    t
}

unsafe fn shgeti(lib: &Lib, t: *mut u8, elemsize: usize, k: *mut c_char) -> (*mut u8, isize) {
    let t = (lib.hmget_key)(
        t as *mut c_void,
        elemsize,
        k as *mut c_void,
        KEYSIZE,
        HM_STRING,
    ) as *mut u8;
    (t, temp_of(t, elemsize))
}

unsafe fn shdel(lib: &Lib, t: *mut u8, elemsize: usize, k: *mut c_char) -> (*mut u8, isize) {
    let t = (lib.hmdel_key)(
        t as *mut c_void,
        elemsize,
        k as *mut c_void,
        KEYSIZE,
        0,
        HM_STRING,
    ) as *mut u8;
    let res = if t.is_null() {
        0
    } else {
        temp_of(t, elemsize)
    };
    (t, res)
}

#[derive(Clone, Copy, Debug)]
enum SOp {
    Put(u32),
    Get(u32),
    Del(u32),
}

fn key_text(n: u32) -> String {
    match n % 5 {
        0 => format!("test_{n}"),
        1 => format!("k{n}"),
        2 => format!("a_rather_longer_key_name_number_{n}"),
        3 => format!("{n}"),
        _ => format!("dup_{}", n % 7),
    }
}

fn value_bytes(vsize: usize, n: u32) -> Vec<u8> {
    (0..vsize)
        .map(|i| (n as u8).wrapping_mul(11).wrapping_add(i as u8))
        .collect()
}

/// `mode < 0` means "no `sh_new_*` call": the map starts null and stb_ds picks
/// `STBDS_SH_DEFAULT` on the first string-mode insert.
fn run_string_script(
    c: &Lib,
    r: &Lib,
    elemsize: usize,
    mode: c_int,
    seed: usize,
    ops: &[SOp],
    tag: &str,
) {
    let _guard = serial();
    let vsize = elemsize - KEYSIZE;
    let mut keys = Keys::new();
    unsafe {
        (c.rand_seed)(seed);
        (r.rand_seed)(seed);

        let (mut ct, mut rt): (*mut u8, *mut u8) = if mode < 0 {
            (std::ptr::null_mut(), std::ptr::null_mut())
        } else {
            (
                (c.shmode_func)(elemsize, mode) as *mut u8,
                (r.shmode_func)(elemsize, mode) as *mut u8,
            )
        };
        assert_eq!(
            snapshot(ct, elemsize, true),
            snapshot(rt, elemsize, true),
            "shmode_func mismatch: {tag} elemsize={elemsize} mode={mode}"
        );

        for (i, op) in ops.iter().enumerate() {
            let ctx = format!(
                "{tag} elemsize={elemsize} mode={mode} seed={seed:#x} step={i} op={op:?}"
            );
            // Set when this step performed a genuine string-mode insert, i.e.
            // the only situation in which `temp_key` holds a defined value.
            let mut inserted_key: Option<String> = None;
            match *op {
                SOp::Put(n) => {
                    let text = key_text(n);
                    let k = keys.make(&text);
                    let v = value_bytes(vsize, n);
                    let len_before = if ct.is_null() {
                        0
                    } else {
                        (*header(ct.sub(elemsize))).length
                    };
                    ct = shput(c, ct, elemsize, k, &v);
                    rt = shput(r, rt, elemsize, k, &v);
                    if (*header(ct.sub(elemsize))).length > len_before {
                        inserted_key = Some(text);
                    }
                }
                SOp::Get(n) => {
                    let k = keys.make(&key_text(n));
                    let (nct, ci) = shgeti(c, ct, elemsize, k);
                    let (nrt, ri) = shgeti(r, rt, elemsize, k);
                    ct = nct;
                    rt = nrt;
                    assert_eq!(ci, ri, "shgeti mismatch: {ctx}");
                }
                SOp::Del(n) => {
                    let k = keys.make(&key_text(n));
                    let (nct, cr) = shdel(c, ct, elemsize, k);
                    let (nrt, rr) = shdel(r, rt, elemsize, k);
                    ct = nct;
                    rt = nrt;
                    assert_eq!(cr, rr, "shdel mismatch: {ctx}");
                }
            }
            let cs = snapshot(ct, elemsize, true);
            let rs = snapshot(rt, elemsize, true);
            assert_eq!(cs, rs, "string map state mismatch: {ctx}");

            // `stbds_make_hash_index` leaves `temp_key` uninitialised and the
            // `STBDS_SH_NONE` insert path never writes it, so only inspect it
            // after an insert into a map whose string mode assigns it.
            if let Some(text) = inserted_key {
                if cs.arena_mode != SH_NONE as u8 {
                    let ck = temp_key_str(ct, elemsize);
                    let rk = temp_key_str(rt, elemsize);
                    assert_eq!(ck, rk, "temp_key mismatch: {ctx}");
                    assert_eq!(
                        ck.as_deref(),
                        Some(text.as_bytes()),
                        "temp_key should hold the inserted key: {ctx}"
                    );
                }
            }
        }

        if !ct.is_null() {
            (c.hmfree_func)(ct.sub(elemsize) as *mut c_void, elemsize);
        }
        if !rt.is_null() {
            (r.hmfree_func)(rt.sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// String-mode maps that stb_ds can actually produce: either no `sh_new_*` call
/// (stb_ds then picks `STBDS_SH_DEFAULT` itself) or one of the three modes that
/// store the key as a `char *`.
///
/// `STBDS_SH_NONE` is deliberately excluded: with `STBDS_HM_STRING` its insert
/// path `memcpy`s the key *bytes* inline while `stbds_is_key_equal` reads the
/// same bytes back as a `char *`, so the C library itself dereferences garbage.
/// That mode is covered by `sh_none_map_with_binary_ops_matches` below.
const MODES: &[c_int] = &[-1, SH_DEFAULT, SH_STRDUP, SH_ARENA];
const ELEMSIZES: &[usize] = &[16, 24, 32];

#[test]
fn shmode_func_fresh_table_matches() {
    for &m in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &es in ELEMSIZES {
            let (c, r) = both();
            run_string_script(&c, &r, es, m, 0x31415926, &[], "shmode-fresh");
        }
    }
}

#[test]
fn sh_sequential_insert_matches() {
    let (c, r) = both();
    let ops: Vec<SOp> = (0..150u32).map(SOp::Put).collect();
    for &m in MODES {
        for &es in ELEMSIZES {
            run_string_script(&c, &r, es, m, 0x31415926, &ops, "sh-sequential");
        }
    }
}

#[test]
fn sh_insert_lookup_delete_matches() {
    let (c, r) = both();
    let mut ops: Vec<SOp> = (0..60u32).map(SOp::Put).collect();
    for n in 0..90u32 {
        ops.push(SOp::Get(n));
    }
    for n in (0..60u32).step_by(2) {
        ops.push(SOp::Del(n));
        ops.push(SOp::Get(n));
    }
    for n in 0..60u32 {
        ops.push(SOp::Put(n));
    }
    for n in 0..60u32 {
        ops.push(SOp::Del(n));
    }
    for &m in MODES {
        for &es in ELEMSIZES {
            run_string_script(&c, &r, es, m, 0x31415926, &ops, "sh-crud");
        }
    }
}

#[test]
fn sh_grow_shrink_and_rebuild_matches() {
    let (c, r) = both();
    // enough inserts to double the table several times, then bulk deletion to
    // trigger the tombstone-rebuild and shrink branches.
    let mut ops: Vec<SOp> = (0..260u32).map(SOp::Put).collect();
    for n in 0..260u32 {
        ops.push(SOp::Del(n));
    }
    for n in 260..320u32 {
        ops.push(SOp::Put(n));
        ops.push(SOp::Get(n));
    }
    for &m in MODES {
        run_string_script(&c, &r, 24, m, 0x31415926, &ops, "sh-grow-shrink");
    }
}

#[test]
fn sh_random_workload_matches() {
    let (c, r) = both();
    for &seed in &[0usize, 0x31415926, usize::MAX] {
        let mut st = seed as u64 ^ 0xA5A5_5A5A_1234_4321;
        let mut ops: Vec<SOp> = Vec::new();
        for _ in 0..600 {
            st = st
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let x = st >> 11;
            let n = (x % 90) as u32;
            match x % 8 {
                0..=3 => ops.push(SOp::Put(n)),
                4..=5 => ops.push(SOp::Get(n)),
                _ => ops.push(SOp::Del(n)),
            }
        }
        for &m in MODES {
            run_string_script(&c, &r, 24, m, seed, &ops, "sh-random");
        }
    }
}

#[test]
fn sh_arena_block_growth_matches() {
    let (c, r) = both();
    // long keys force the arena to allocate fresh blocks, including the
    // oversized-string branch of stbds_stralloc.
    let mut keys = Keys::new();
    let _guard = serial();
    unsafe {
        (c.rand_seed)(0x31415926);
        (r.rand_seed)(0x31415926);
        let elemsize = 24usize;
        let mut ct = (c.shmode_func)(elemsize, SH_ARENA) as *mut u8;
        let mut rt = (r.shmode_func)(elemsize, SH_ARENA) as *mut u8;
        for i in 0..80usize {
            let s: String = std::iter::repeat('k')
                .take(1 + i * 37)
                .collect::<String>()
                + &i.to_string();
            let k = keys.make(&s);
            let v = value_bytes(elemsize - KEYSIZE, i as u32);
            ct = shput(&c, ct, elemsize, k, &v);
            rt = shput(&r, rt, elemsize, k, &v);
            assert_eq!(
                snapshot(ct, elemsize, true),
                snapshot(rt, elemsize, true),
                "arena-mode string map mismatch at step {i} (key len {})",
                s.len()
            );
        }
        (c.hmfree_func)(ct.sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)(rt.sub(elemsize) as *mut c_void, elemsize);
    }
}

#[test]
fn sh_keys_with_high_bit_bytes_match() {
    let (c, r) = both();
    let _guard = serial();
    unsafe {
        for &m in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            (c.rand_seed)(7);
            (r.rand_seed)(7);
            let elemsize = 16usize;
            let mut ct = (c.shmode_func)(elemsize, m) as *mut u8;
            let mut rt = (r.shmode_func)(elemsize, m) as *mut u8;
            let mut bufs: Vec<Box<Vec<u8>>> = Vec::new();
            for i in 0..60usize {
                let mut b = Box::new(vec![0u8; 0]);
                for j in 0..(1 + i % 17) {
                    b.push(0x80u8.wrapping_add((i * 7 + j) as u8) | 0x80);
                }
                b.push(0);
                let k = b.as_mut_ptr() as *mut c_char;
                bufs.push(b);
                let v = value_bytes(elemsize - KEYSIZE, i as u32);
                ct = shput(&c, ct, elemsize, k, &v);
                rt = shput(&r, rt, elemsize, k, &v);
                assert_eq!(
                    snapshot(ct, elemsize, true),
                    snapshot(rt, elemsize, true),
                    "high-bit string key mismatch mode={m} step={i}"
                );
                let (nct, ci) = shgeti(&c, ct, elemsize, k);
                let (nrt, ri) = shgeti(&r, rt, elemsize, k);
                ct = nct;
                rt = nrt;
                assert_eq!(ci, ri, "high-bit shgeti mismatch mode={m} step={i}");
            }
            (c.hmfree_func)(ct.sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)(rt.sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

/// A map created with `stbds_shmode_func(.., STBDS_SH_NONE)` keeps keys inline,
/// so it is only usable through the binary-key path.
#[test]
fn sh_none_map_with_binary_ops_matches() {
    let (c, r) = both();
    let _guard = serial();
    let elemsize = 16usize;
    let keysize = 8usize;
    unsafe {
        (c.rand_seed)(0x31415926);
        (r.rand_seed)(0x31415926);
        let mut ct = (c.shmode_func)(elemsize, SH_NONE) as *mut u8;
        let mut rt = (r.shmode_func)(elemsize, SH_NONE) as *mut u8;
        assert_eq!(
            snapshot(ct, elemsize, false),
            snapshot(rt, elemsize, false),
            "shmode_func(SH_NONE) fresh state mismatch"
        );
        for n in 0..120i64 {
            let k = n.to_le_bytes();
            let v = (n * 977).to_le_bytes();
            ct = hmput_bytes(&c, ct, elemsize, &k, &v);
            rt = hmput_bytes(&r, rt, elemsize, &k, &v);
            assert_eq!(
                snapshot(ct, elemsize, false),
                snapshot(rt, elemsize, false),
                "SH_NONE binary put mismatch at {n}"
            );
        }
        for n in 0..120i64 {
            let k = n.to_le_bytes();
            let (nct, ci) = hmgeti_bytes(&c, ct, elemsize, &k);
            let (nrt, ri) = hmgeti_bytes(&r, rt, elemsize, &k);
            ct = nct;
            rt = nrt;
            assert_eq!(ci, ri, "SH_NONE binary get mismatch at {n}");
        }
        for n in (0..120i64).step_by(2) {
            let k = n.to_le_bytes();
            let (nct, cr) = hmdel_bytes(&c, ct, elemsize, &k);
            let (nrt, rr) = hmdel_bytes(&r, rt, elemsize, &k);
            ct = nct;
            rt = nrt;
            assert_eq!(cr, rr, "SH_NONE binary del mismatch at {n}");
            assert_eq!(
                snapshot(ct, elemsize, false),
                snapshot(rt, elemsize, false),
                "SH_NONE binary del state mismatch at {n}"
            );
        }
        (c.hmfree_func)(ct.sub(elemsize) as *mut c_void, elemsize);
        (r.hmfree_func)(rt.sub(elemsize) as *mut c_void, elemsize);
    }
    let _ = keysize;
}
