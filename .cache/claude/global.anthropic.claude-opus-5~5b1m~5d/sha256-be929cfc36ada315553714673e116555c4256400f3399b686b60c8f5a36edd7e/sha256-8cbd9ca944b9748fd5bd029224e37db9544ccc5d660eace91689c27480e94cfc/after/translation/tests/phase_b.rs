//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//! Both implementations are exercised only through their `.so` exports.

mod common;

use common::driver::*;
use common::*;
use std::ffi::{c_int, c_void, CString};

const SEED: u64 = 0xC0FFEE;

fn libs() -> (Lib, Lib) {
    both()
}

// --------------------------------------------------------------------------
// rows 1-3: stbds_hash_bytes
// --------------------------------------------------------------------------

#[test]
fn cfg_01_hash_bytes_all_lengths() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED);
    unsafe {
        for len in 0..=64usize {
            for _ in 0..64 {
                let mut buf = rng.bytes(len.max(1));
                let hc = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 0x31415926);
                let hr = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 0x31415926);
                assert_eq!(hc, hr, "hash_bytes len={len} buf={buf:?}");
            }
        }
    }
}

#[test]
fn cfg_02_hash_bytes_high_bit_bytes() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 2);
    unsafe {
        for len in 0..=64usize {
            let cases: Vec<Vec<u8>> = vec![
                vec![0xFFu8; len.max(1)],
                vec![0x80u8; len.max(1)],
                (0..len.max(1)).map(|i| 0x80 | (i as u8)).collect(),
                (0..len.max(1)).map(|_| 0x80 | (rng.next_u32() as u8)).collect(),
            ];
            for mut buf in cases {
                let hc = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 0);
                let hr = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, 0);
                assert_eq!(hc, hr, "hash_bytes high-bit len={len}");
            }
        }
    }
}

#[test]
fn cfg_03_hash_bytes_seeds() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 3);
    let mut seeds: Vec<usize> = vec![0, 1, 0x31415926, usize::MAX, usize::MAX - 1, 1 << 63];
    for _ in 0..8 {
        seeds.push(rng.next_u64() as usize);
    }
    unsafe {
        for len in 0..=64usize {
            let mut buf = rng.bytes(len.max(1));
            for &s in &seeds {
                let hc = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, s);
                let hr = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, s);
                assert_eq!(hc, hr, "hash_bytes len={len} seed={s:#x}");
            }
        }
    }
}

// --------------------------------------------------------------------------
// rows 4-6: stbds_hash_string
// --------------------------------------------------------------------------

#[test]
fn cfg_04_hash_string_ascii() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 4);
    unsafe {
        for len in 0..=64usize {
            for _ in 0..32 {
                let s = rng.ascii(len);
                let p = s.as_ptr() as *mut i8;
                assert_eq!(
                    (c.hash_string)(p, 0x31415926),
                    (r.hash_string)(p, 0x31415926),
                    "hash_string {s:?}"
                );
            }
        }
    }
}

#[test]
fn cfg_05_hash_string_high_bytes() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 5);
    unsafe {
        let empty = CString::new("").unwrap();
        assert_eq!(
            (c.hash_string)(empty.as_ptr() as *mut i8, 12345),
            (r.hash_string)(empty.as_ptr() as *mut i8, 12345)
        );
        for len in 1..=64usize {
            for _ in 0..32 {
                let s = rng.high_bytes_string(len);
                let p = s.as_ptr() as *mut i8;
                assert_eq!((c.hash_string)(p, 7), (r.hash_string)(p, 7), "hash_string {s:?}");
            }
        }
    }
}

#[test]
fn cfg_06_hash_string_seeds() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 6);
    let mut seeds: Vec<usize> = vec![0, 1, usize::MAX, 1 << 63];
    for _ in 0..8 {
        seeds.push(rng.next_u64() as usize);
    }
    unsafe {
        for len in 0..=32usize {
            let s = rng.ascii(len);
            let p = s.as_ptr() as *mut i8;
            for &sd in &seeds {
                assert_eq!((c.hash_string)(p, sd), (r.hash_string)(p, sd), "len={len} seed={sd:#x}");
            }
        }
    }
}

// --------------------------------------------------------------------------
// row 7: stbds_rand_seed influences the map seed sequence
// --------------------------------------------------------------------------

#[test]
fn cfg_07_rand_seed_sequence() {
    let (c, r) = libs();
    unsafe {
        for s in [0usize, 1, usize::MAX, 0x31415926, 0xdead_beef] {
            seed_both(&c, &r, s);
            // Each shmode_func call consumes and mutates the global seed; the
            // seeds baked into successive tables must match step for step.
            let mut cs = String::new();
            let mut rs = String::new();
            for _ in 0..8 {
                let ct = (c.shmode_func)(16, 2);
                let rt = (r.shmode_func)(16, 2);
                let ch = map_header(ct, 16);
                let rh = map_header(rt, 16);
                cs.push_str(&format!("{:#x}\n", (*(ch.hash_table as *mut HashIndex)).seed));
                rs.push_str(&format!("{:#x}\n", (*(rh.hash_table as *mut HashIndex)).seed));
                (c.hmfree_func)((ct as *mut u8).sub(16) as *mut c_void, 16);
                (r.hmfree_func)((rt as *mut u8).sub(16) as *mut c_void, 16);
            }
            assert_same(&format!("seed sequence from {s:#x}"), &cs, &rs);
        }
    }
}

// --------------------------------------------------------------------------
// rows 8-11: stbds_arrgrowf / stbds_arrfreef
// --------------------------------------------------------------------------

unsafe fn grow_snap(a: *mut c_void) -> String {
    if a.is_null() {
        return "NULL".to_string();
    }
    let h = header(a);
    format!("len={} cap={} temp={} table_null={}", h.length, h.capacity, h.temp, h.hash_table.is_null())
}

/// `stbds_arrfreef` on NULL is UB in both implementations (it frees
/// `(header*)NULL - 1`); skip it, as the C's own callers never do that.
unsafe fn free_if(f: ArrFreeF, a: *mut c_void) {
    if !a.is_null() {
        f(a);
    }
}

#[test]
fn cfg_08_arrgrowf_from_null() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [1usize, 4, 8, 12, 40] {
            for addlen in [0usize, 1, 7] {
                for min_cap in [0usize, 1, 3, 4, 5, 1000] {
                    let ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    assert_same(
                        &format!("arrgrowf(NULL,{elemsize},{addlen},{min_cap})"),
                        &grow_snap(ca),
                        &grow_snap(ra),
                    );
                    free_if(c.arrfreef, ca);
                    free_if(r.arrfreef, ra);
                }
            }
        }
    }
}

#[test]
fn cfg_09_arrgrowf_growth_chain() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [1usize, 8, 40] {
            let mut ca: *mut c_void = std::ptr::null_mut();
            let mut ra: *mut c_void = std::ptr::null_mut();
            let mut cs = String::new();
            let mut rs = String::new();
            for step in 0..512 {
                // emulate arrmaybegrow(a,1) + length++
                let ch = if ca.is_null() { None } else { Some(header(ca)) };
                let need = ch.map(|h| h.length + 1 > h.capacity).unwrap_or(true);
                if need {
                    ca = (c.arrgrowf)(ca, elemsize, 1, 0);
                    ra = (r.arrgrowf)(ra, elemsize, 1, 0);
                }
                (*((ca as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader)).length += 1;
                (*((ra as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader)).length += 1;
                cs.push_str(&format!("{step}:{}\n", grow_snap(ca)));
                rs.push_str(&format!("{step}:{}\n", grow_snap(ra)));
            }
            assert_same(&format!("arrgrowf chain elemsize={elemsize}"), &cs, &rs);
            free_if(c.arrfreef, ca);
            free_if(r.arrfreef, ra);
        }
    }
}

#[test]
fn cfg_10_arrgrowf_no_grow_and_one_past() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [4usize, 8, 24] {
            let mut ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let mut ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let cap = header(ca).capacity;
            assert_eq!(cap, header(ra).capacity);
            for min_cap in [0usize, 1, cap - 1, cap, cap + 1] {
                let cap_now = header(ca).capacity;
                let ca2 = (c.arrgrowf)(ca, elemsize, 0, min_cap);
                let ra2 = (r.arrgrowf)(ra, elemsize, 0, min_cap);
                if min_cap <= cap_now {
                    // fast path: the C returns `a` itself, unmodified
                    assert_eq!(ca2, ca, "C: no-grow path reallocated");
                    assert_eq!(ra2, ra, "Rust: no-grow path reallocated");
                }
                ca = ca2;
                ra = ra2;
                assert_same(
                    &format!("arrgrowf(a,{elemsize},0,{min_cap})"),
                    &grow_snap(ca),
                    &grow_snap(ra),
                );
            }
            free_if(c.arrfreef, ca);
            free_if(r.arrfreef, ra);
        }
    }
}

#[test]
fn cfg_11_arrgrowf_elemsize_zero() {
    let (c, r) = libs();
    unsafe {
        for min_cap in [0usize, 1, 4, 9, 1000] {
            let ca = (c.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            assert_same(
                &format!("arrgrowf(NULL,0,0,{min_cap})"),
                &grow_snap(ca),
                &grow_snap(ra),
            );
            free_if(c.arrfreef, ca);
            free_if(r.arrfreef, ra);
        }
    }
}

// --------------------------------------------------------------------------
// rows 12-24: binary hash maps
// --------------------------------------------------------------------------

fn k4(v: u32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

#[test]
fn cfg_12_put_single_binary() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "row12");
        p.put_bin(&k4(1234), &k4(7));
        p.check("after 1 put");
        assert_eq!(p.get_bin(&k4(1234)), 0);
        p.check_value(0, 4);
        p.free();
    }
}

#[test]
fn cfg_13_put_across_first_growth() {
    let (c, r) = libs();
    unsafe {
        for n in [5usize, 6, 7] {
            seed_both(&c, &r, 0x31415926);
            let mut rng = Rng::new(SEED ^ n as u64);
            let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, &format!("row13 n={n}"));
            for i in 0..n {
                let key = k4(rng.next_u32());
                p.put_bin(&key, &k4(i as u32));
                p.check(&format!("put {i}"));
            }
            p.free();
        }
    }
}

#[test]
fn cfg_14_put_many_binary() {
    let (c, r) = libs();
    unsafe {
        for n in [12usize, 13, 50, 200, 2000] {
            seed_both(&c, &r, 0x31415926);
            let mut rng = Rng::new(SEED ^ (n as u64) << 8);
            let mut keys = Vec::new();
            let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, &format!("row14 n={n}"));
            for i in 0..n {
                let key = k4(rng.next_u32());
                keys.push(key.clone());
                p.put_bin(&key, &k4(i as u32));
            }
            p.check("after all puts");
            for key in &keys {
                p.get_bin(key);
            }
            p.check("after all gets");
            p.free();
        }
    }
}

#[test]
fn cfg_15_put_duplicate_keys() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 15);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "row15");
        for i in 0..2000u32 {
            let key = k4(rng.next_u32() % 64);
            p.put_bin(&key, &k4(i));
            if i % 97 == 0 {
                p.check(&format!("dup put {i}"));
            }
        }
        p.check("after dup puts");
        p.free();
    }
}

#[test]
fn cfg_16_wider_keys() {
    let (c, r) = libs();
    unsafe {
        for (elemsize, keysize) in [(16usize, 8usize), (24, 16), (40, 32)] {
            seed_both(&c, &r, 0x31415926);
            let mut rng = Rng::new(SEED ^ elemsize as u64);
            let mut p = Pair::empty(
                &c,
                &r,
                elemsize,
                keysize,
                0,
                KeyKind::Inline,
                &format!("row16 {elemsize}/{keysize}"),
            );
            for i in 0..300 {
                let key = rng.bytes(keysize);
                let val = rng.bytes(elemsize - keysize);
                p.put_bin(&key, &val);
                if i % 41 == 0 {
                    p.check(&format!("put {i}"));
                }
            }
            p.check("after puts");
            p.free();
        }
    }
}

#[test]
fn cfg_17_subword_keys() {
    let (c, r) = libs();
    unsafe {
        for keysize in [1usize, 2] {
            seed_both(&c, &r, 0x31415926);
            let mut rng = Rng::new(SEED ^ keysize as u64);
            let mut p = Pair::empty(
                &c,
                &r,
                8,
                keysize,
                0,
                KeyKind::Inline,
                &format!("row17 keysize={keysize}"),
            );
            for i in 0..500 {
                let key = rng.bytes(keysize);
                let val = rng.bytes(8 - keysize);
                p.put_bin(&key, &val);
                if i % 37 == 0 {
                    p.check(&format!("put {i}"));
                }
            }
            p.check("after puts");
            p.free();
        }
    }
}

#[test]
fn cfg_18_negative_mode_is_binary() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 18);
        let mut p = Pair::empty(&c, &r, 8, 4, -1, KeyKind::Inline, "row18 mode=-1");
        let mut keys = Vec::new();
        for i in 0..200u32 {
            let key = k4(rng.next_u32());
            keys.push(key.clone());
            p.put_bin(&key, &k4(i));
        }
        p.check("after puts (mode=-1)");
        for k in &keys {
            p.get_bin(k);
        }
        p.check("after gets (mode=-1)");
        p.free();
    }
}

#[test]
fn cfg_19_get_present_and_absent() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 19);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "row19");
        let mut keys = std::collections::HashSet::new();
        for i in 0..200u32 {
            let kv = rng.next_u32() | 1; // keep 0 free for "absent"
            keys.insert(kv);
            p.put_bin(&k4(kv), &k4(i));
        }
        for &kv in &keys {
            assert!(p.get_bin(&k4(kv)) >= 0, "present key {kv} not found");
        }
        for _ in 0..200 {
            let kv = rng.next_u32() & !1u32;
            if keys.contains(&kv) {
                continue;
            }
            assert_eq!(p.get_bin(&k4(kv)), -1, "absent key {kv} found");
        }
        p.check("after mixed gets");
        p.free();
    }
}

#[test]
fn cfg_20_get_ts_out_param() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 20);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "row20");
        let mut keys = Vec::new();
        for i in 0..200u32 {
            let kv = rng.next_u32() | 1;
            keys.push(kv);
            p.put_bin(&k4(kv), &k4(i));
        }
        // the header temp must be untouched by the _ts flavour
        let before_c = p.temp(p.ct);
        let before_r = p.temp(p.rt);
        assert_eq!(before_c, before_r);
        for &kv in &keys {
            assert!(p.get_bin_ts(&k4(kv)) >= 0);
        }
        for _ in 0..200 {
            let kv = rng.next_u32() & !1u32;
            let t = p.get_bin_ts(&k4(kv));
            assert!(t == -1 || t >= 0);
        }
        assert_eq!(p.temp(p.ct), before_c, "C: _ts modified header temp");
        assert_eq!(p.temp(p.rt), before_r, "Rust: _ts modified header temp");
        p.check("after _ts gets");
        p.free();
    }
}

#[test]
fn cfg_21_delete_third_then_reget() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 21);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "row21");
        let mut keys = Vec::new();
        for i in 0..200u32 {
            let kv = rng.next_u32() | 1;
            keys.push(kv);
            p.put_bin(&k4(kv), &k4(i));
        }
        for (i, &kv) in keys.iter().enumerate() {
            if i % 3 == 0 {
                p.del_bin(&k4(kv), 0);
                p.check(&format!("del {i}"));
            }
        }
        for &kv in &keys {
            p.get_bin(&k4(kv));
        }
        p.check("after deletes+gets");
        p.free();
    }
}

#[test]
fn cfg_22_delete_all_shrink_chain() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 22);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "row22");
        let mut keys = Vec::new();
        for i in 0..200u32 {
            let kv = rng.next_u32() | 1;
            if keys.contains(&kv) {
                continue;
            }
            keys.push(kv);
            p.put_bin(&k4(kv), &k4(i));
        }
        p.check("before deleting everything");
        for (i, &kv) in keys.iter().enumerate() {
            p.del_bin(&k4(kv), 0);
            p.check(&format!("del all step {i}"));
        }
        p.free();
    }
}

#[test]
fn cfg_23_interleaved_put_get_del() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 23);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "row23");
        for i in 0..2000u32 {
            let kv = (rng.next_u32() % 64) | 1;
            match rng.below(3) {
                0 => p.put_bin(&k4(kv), &k4(i)),
                1 => {
                    p.get_bin(&k4(kv));
                }
                _ => {
                    p.del_bin(&k4(kv), 0);
                }
            }
            if i % 53 == 0 {
                p.check(&format!("mixed op {i}"));
            }
        }
        p.check("after mixed ops");
        p.free();
    }
}

#[test]
fn cfg_24_delete_with_keyoffset() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 24);
        // elemsize 16, key at offset 8, keysize 4.  Emulates a struct whose
        // key member is not first: put uses keyoffset 0 implicitly (hmput_key
        // hard-codes keyoffset=0), so drive hmdel_key with keyoffset=0 for
        // consistency, and separately verify keyoffset=8 rejects/behaves alike.
        let mut p = Pair::empty(&c, &r, 16, 4, 0, KeyKind::Inline, "row24");
        let mut keys = Vec::new();
        for _i in 0..100u32 {
            let kv = rng.next_u32() | 1;
            keys.push(kv);
            let v = rng.bytes(12);
            p.put_bin(&k4(kv), &v);
        }
        p.check("after puts");
        // keyoffset = 8: reads the *value* bytes as the key, so almost nothing
        // is found; whatever happens, both must agree.
        for &kv in keys.iter().take(20) {
            p.del_bin(&k4(kv), 8);
        }
        p.check("after keyoffset=8 deletes");
        for &kv in keys.iter().skip(20) {
            p.del_bin(&k4(kv), 0);
        }
        p.check("after keyoffset=0 deletes");
        p.free();
    }
}

// --------------------------------------------------------------------------
// rows 25-32: string maps
// --------------------------------------------------------------------------

#[test]
fn cfg_25_string_map_implicit_default_mode() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 25);
        let mut p = Pair::empty(&c, &r, 16, 8, 1, KeyKind::Pointer, "row25");
        let mut keys: Vec<CString> = Vec::new();
        for i in 0..200 {
            let s = rng.ascii_len(1, 20);
            keys.push(s);
            let k = keys.last().unwrap().clone();
            p.put_str(&k, &(i as u64).to_le_bytes());
            keys.pop();
            keys.push(k);
        }
        p.check("after string puts (SH_DEFAULT)");
        for k in &keys {
            p.get_str(k);
        }
        p.check("after string gets");
        p.free();
    }
}

#[test]
fn cfg_26_string_map_duplicates() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 26);
        let mut p = Pair::empty(&c, &r, 16, 8, 1, KeyKind::Pointer, "row26");
        let domain: Vec<CString> = (0..24).map(|_| rng.ascii_len(1, 10)).collect();
        for i in 0..600u64 {
            let k = &domain[rng.below(domain.len())];
            p.put_str(k, &i.to_le_bytes());
            if i % 29 == 0 {
                p.check(&format!("dup string put {i}"));
            }
        }
        p.check("after duplicate string puts");
        p.free();
    }
}

unsafe fn string_map_case(
    c: &Lib,
    r: &Lib,
    sh_mode: c_int,
    mode: c_int,
    kind: KeyKind,
    n: usize,
    maxlen: usize,
    label: &str,
    delete: bool,
) {
    seed_both(c, r, 0x31415926);
    let mut rng = Rng::new(SEED ^ (sh_mode as u64) ^ ((mode as u64) << 8) ^ (n as u64) << 16);
    let mut p = Pair::shmode(c, r, 16, 8, mode, sh_mode, kind, label);
    let keys: Vec<CString> = (0..n).map(|_| rng.ascii_len(0, maxlen)).collect();
    for (i, k) in keys.iter().enumerate() {
        p.put_str(k, &(i as u64).to_le_bytes());
        if i % 37 == 0 {
            p.check(&format!("put {i}"));
        }
    }
    p.check("after puts");
    for k in &keys {
        p.get_str(k);
    }
    p.check("after gets");
    if delete {
        for (i, k) in keys.iter().enumerate() {
            p.del_str(k);
            if i % 31 == 0 {
                p.check(&format!("del {i}"));
            }
        }
        p.check("after deletes");
    }
    p.free();
}

#[test]
fn cfg_27_sh_strdup() {
    let (c, r) = libs();
    unsafe {
        string_map_case(&c, &r, 2, 1, KeyKind::Pointer, 200, 40, "row27 SH_STRDUP", false);
    }
}

#[test]
fn cfg_28_sh_strdup_with_deletes() {
    let (c, r) = libs();
    unsafe {
        string_map_case(&c, &r, 2, 1, KeyKind::Pointer, 200, 40, "row28 SH_STRDUP+del", true);
    }
}

#[test]
fn cfg_29_sh_arena() {
    let (c, r) = libs();
    unsafe {
        string_map_case(&c, &r, 3, 1, KeyKind::Pointer, 500, 40, "row29 SH_ARENA", false);
        // long strings force the oversize-block path inside stbds_stralloc
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 29);
        let mut p = Pair::shmode(&c, &r, 16, 8, 1, 3, KeyKind::Pointer, "row29 SH_ARENA long");
        let mut keys: Vec<CString> = Vec::new();
        for i in 0..40 {
            let len = if i % 4 == 0 { 600 + rng.below(2000) } else { 1 + rng.below(30) };
            keys.push(rng.ascii(len));
        }
        for (i, k) in keys.iter().enumerate() {
            p.put_str(k, &(i as u64).to_le_bytes());
            p.check(&format!("long arena put {i}"));
        }
        for k in &keys {
            p.get_str(k);
        }
        p.check("long arena gets");
        p.free();
    }
}

#[test]
fn cfg_30_sh_none_with_string_mode() {
    let (c, r) = libs();
    unsafe {
        // string.mode == 0 => `default:` memcpy of keysize bytes, but mode==1
        // means the *comparison* uses strcmp on the stored 8 bytes.  Keys must
        // be 8-byte NUL-terminated strings so the memcpy'd bytes stay a valid
        // C string.
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 30);
        let mut p = Pair::shmode(&c, &r, 16, 8, 1, 0, KeyKind::Inline, "row30 SH_NONE+mode1");
        let mut keys: Vec<CString> = Vec::new();
        for _ in 0..40 {
            keys.push(rng.ascii_len(1, 6));
        }
        for (i, k) in keys.iter().enumerate() {
            p.put_str(k, &(i as u64).to_le_bytes());
            p.check(&format!("SH_NONE put {i}"));
        }
        p.check("after SH_NONE puts");
        p.free();
    }
}

#[test]
fn cfg_31_mode_two_string_path() {
    let (c, r) = libs();
    unsafe {
        // mode == 2 is >= STBDS_HM_STRING so it takes the string path, but
        // hmdel_key's strdup-free is guarded by `mode == STBDS_HM_STRING`.
        // NB: deleting with mode==2 aborts in the C (see ERRORS.md row 44), so
        // this row covers put/get only; the abort is a Phase C differential.
        string_map_case(&c, &r, 2, 2, KeyKind::Pointer, 150, 30, "row31 mode=2 STRDUP", false);
        string_map_case(&c, &r, 3, 2, KeyKind::Pointer, 150, 30, "row31 mode=2 ARENA", false);
        string_map_case(&c, &r, 1, 2, KeyKind::Pointer, 150, 30, "row31 mode=2 DEFAULT", false);
    }
}

#[test]
fn cfg_32_shmode_func_elemsizes() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [8usize, 16, 24, 40] {
            for sh_mode in [0, 1, 2, 3] {
                seed_both(&c, &r, 0x31415926);
                let ct = (c.shmode_func)(elemsize, sh_mode);
                let rt = (r.shmode_func)(elemsize, sh_mode);
                assert_same(
                    &format!("shmode_func({elemsize},{sh_mode})"),
                    &snapshot_map(ct, elemsize, 8, KeyKind::Inline),
                    &snapshot_map(rt, elemsize, 8, KeyKind::Inline),
                );
                (c.hmfree_func)((ct as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (r.hmfree_func)((rt as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

// --------------------------------------------------------------------------
// rows 33-35
// --------------------------------------------------------------------------

#[test]
fn cfg_33_hmput_default_then_put() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [8usize, 16, 24] {
            seed_both(&c, &r, 0x31415926);
            let mut p = Pair::empty(&c, &r, elemsize, 4, 0, KeyKind::Inline, "row33");
            p.put_default();
            p.check("after hmput_default on NULL");
            let ct1 = p.ct;
            let rt1 = p.rt;
            p.put_default();
            assert_eq!(p.ct, ct1, "C: second hmput_default moved the map");
            assert_eq!(p.rt, rt1, "Rust: second hmput_default moved the map");
            p.check("after second hmput_default");
            let mut rng = Rng::new(SEED ^ 33);
            for i in 0..50u32 {
                p.put_bin(&k4(rng.next_u32()), &rng.bytes(elemsize - 4));
                let _ = i;
            }
            p.check("after puts on a defaulted map");
            p.free();
        }
    }
}

#[test]
fn cfg_34_hmput_default_then_get_no_table() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "row34");
        p.put_default();
        for kv in [0u32, 1, 7, u32::MAX] {
            assert_eq!(p.get_bin(&k4(kv)), -1, "no-table get must yield -1");
            assert_eq!(p.get_bin_ts(&k4(kv)), -1, "no-table get_ts must yield -1");
        }
        p.check("after gets against a table-less map");
        p.free();
    }
}

#[test]
fn cfg_35_hmfree_all_modes() {
    let (c, r) = libs();
    unsafe {
        for sh_mode in [0, 1, 2, 3] {
            for populated in [false, true] {
                seed_both(&c, &r, 0x31415926);
                let mut rng = Rng::new(SEED ^ 35 ^ sh_mode as u64);
                // string.mode == 0 stores the key with `memcpy` (8 raw bytes),
                // so the key field is NOT a pointer in that configuration.
                let kind = if sh_mode == 0 { KeyKind::Inline } else { KeyKind::Pointer };
                let mut p = Pair::shmode(&c, &r, 16, 8, 1, sh_mode, kind, "row35");
                if populated {
                    let maxlen = if sh_mode == 0 { 6 } else { 20 };
                    let keys: Vec<CString> =
                        (0..30).map(|_| rng.ascii_len(1, maxlen)).collect();
                    for (i, k) in keys.iter().enumerate() {
                        p.put_str(k, &(i as u64).to_le_bytes());
                    }
                    p.check(&format!("populated sh_mode={sh_mode}"));
                }
                p.free();
            }
        }
    }
}

// --------------------------------------------------------------------------
// rows 36-40: string arena
// --------------------------------------------------------------------------

#[test]
fn cfg_36_arena_many_small_strings() {
    let (c, r) = libs();
    unsafe {
        let mut rng = Rng::new(SEED ^ 36);
        let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let mut ra = ca;
        let mut cs = String::new();
        let mut rs = String::new();
        for i in 0..300 {
            let s = rng.ascii_len(0, 40);
            let pc = (c.stralloc)(&mut ca as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            let pr = (r.stralloc)(&mut ra as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            cs.push_str(&format!("{i}: {} {}\n", cstr(pc), snapshot_arena(&ca)));
            rs.push_str(&format!("{i}: {} {}\n", cstr(pr), snapshot_arena(&ra)));
        }
        assert_same("arena small strings", &cs, &rs);
        (c.strreset)(&mut ca as *mut _ as *mut c_void);
        (r.strreset)(&mut ra as *mut _ as *mut c_void);
        assert_same("arena after reset", &snapshot_arena(&ca), &snapshot_arena(&ra));
    }
}

#[test]
fn cfg_37_arena_oversize_first() {
    let (c, r) = libs();
    unsafe {
        let mut rng = Rng::new(SEED ^ 37);
        for len in [513usize, 1000, 5000, 100_000] {
            let s = rng.ascii(len);
            let mut ca =
                StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
            let mut ra = ca;
            let pc = (c.stralloc)(&mut ca as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            let pr = (r.stralloc)(&mut ra as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            assert_same(
                &format!("arena oversize-first len={len}"),
                &format!("{} {}", cstr(pc), snapshot_arena(&ca)),
                &format!("{} {}", cstr(pr), snapshot_arena(&ra)),
            );
            // a following small allocation must behave the same
            let t = rng.ascii(10);
            let qc = (c.stralloc)(&mut ca as *mut _ as *mut c_void, t.as_ptr() as *mut i8);
            let qr = (r.stralloc)(&mut ra as *mut _ as *mut c_void, t.as_ptr() as *mut i8);
            assert_same(
                &format!("arena oversize-first follow-up len={len}"),
                &format!("{} {}", cstr(qc), snapshot_arena(&ca)),
                &format!("{} {}", cstr(qr), snapshot_arena(&ra)),
            );
            (c.strreset)(&mut ca as *mut _ as *mut c_void);
            (r.strreset)(&mut ra as *mut _ as *mut c_void);
        }
    }
}

#[test]
fn cfg_38_arena_oversize_splice() {
    let (c, r) = libs();
    unsafe {
        let mut rng = Rng::new(SEED ^ 38);
        let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let mut ra = ca;
        let mut cs = String::new();
        let mut rs = String::new();
        for i in 0..40 {
            let len = if i % 3 == 0 { 700 + rng.below(3000) } else { rng.below(30) };
            let s = rng.ascii(len);
            let pc = (c.stralloc)(&mut ca as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            let pr = (r.stralloc)(&mut ra as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            cs.push_str(&format!("{i}: {} {}\n", cstr(pc), snapshot_arena(&ca)));
            rs.push_str(&format!("{i}: {} {}\n", cstr(pr), snapshot_arena(&ra)));
        }
        assert_same("arena oversize splice", &cs, &rs);
        (c.strreset)(&mut ca as *mut _ as *mut c_void);
        (r.strreset)(&mut ra as *mut _ as *mut c_void);
    }
}

#[test]
fn cfg_39_arena_block_ceiling() {
    let (c, r) = libs();
    unsafe {
        let mut rng = Rng::new(SEED ^ 39);
        let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let mut ra = ca;
        let mut cs = String::new();
        let mut rs = String::new();
        // Each allocation that does not fit bumps `block`; ask for exactly the
        // whole current block each time so `block` climbs past the 1<<20 cap.
        for i in 0..48u32 {
            let want = (512usize << (i as usize / 2)).min(1 << 21);
            let s = rng.ascii(want);
            let pc = (c.stralloc)(&mut ca as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            let pr = (r.stralloc)(&mut ra as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            cs.push_str(&format!("{i}: nonnull={} {}\n", !pc.is_null(), snapshot_arena(&ca)));
            rs.push_str(&format!("{i}: nonnull={} {}\n", !pr.is_null(), snapshot_arena(&ra)));
            assert_eq!(cstr(pc), cstr(pr), "arena ceiling step {i} content");
        }
        assert_same("arena block ceiling", &cs, &rs);
        (c.strreset)(&mut ca as *mut _ as *mut c_void);
        (r.strreset)(&mut ra as *mut _ as *mut c_void);
    }
}

#[test]
fn cfg_40_strreset_variants() {
    let (c, r) = libs();
    unsafe {
        let mut rng = Rng::new(SEED ^ 40);
        // (a) all-zero arena, reset twice
        let mut ca = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let mut ra = ca;
        for _ in 0..2 {
            (c.strreset)(&mut ca as *mut _ as *mut c_void);
            (r.strreset)(&mut ra as *mut _ as *mut c_void);
            assert_same("strreset on empty", &snapshot_arena(&ca), &snapshot_arena(&ra));
        }
        // (b) populated arena, reset, reuse, reset again
        for _ in 0..3 {
            for _ in 0..50 {
                let s = rng.ascii_len(0, 59);
                (c.stralloc)(&mut ca as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
                (r.stralloc)(&mut ra as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
                assert_same("arena reuse", &snapshot_arena(&ca), &snapshot_arena(&ra));
            }
            (c.strreset)(&mut ca as *mut _ as *mut c_void);
            (r.strreset)(&mut ra as *mut _ as *mut c_void);
            assert_same("strreset after use", &snapshot_arena(&ca), &snapshot_arena(&ra));
        }
    }
}

// --------------------------------------------------------------------------
// rows 41-42: strkey / intput
// --------------------------------------------------------------------------

#[test]
fn cfg_41_strkey() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 41);
    let mut ns: Vec<c_int> = vec![0, 1, -1, 9, 11, c_int::MIN, c_int::MAX, 10, 100, -100];
    for _ in 0..200 {
        ns.push(rng.next_u32() as c_int);
    }
    unsafe {
        for n in ns {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            assert_eq!(cstr(pc), cstr(pr), "strkey({n}) text");
            // the whole 256-byte static buffer must match, not just the string
            let bc: Vec<u8> = (0..256).map(|i| *(pc as *const u8).add(i)).collect();
            let br: Vec<u8> = (0..256).map(|i| *(pr as *const u8).add(i)).collect();
            let lc = bc.iter().position(|&b| b == 0).unwrap();
            let lr = br.iter().position(|&b| b == 0).unwrap();
            assert_eq!(lc, lr, "strkey({n}) length");
            assert_eq!(&bc[..=lc], &br[..=lr], "strkey({n}) bytes");
        }
    }
}

#[test]
fn cfg_42_intput_ok_values() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 42);
    let mut ns: Vec<c_int> = vec![c_int::MIN, -1, 0, 1, 8, 10, 12, c_int::MAX, 1000, -1000];
    for _ in 0..200 {
        let v = rng.next_u32() as c_int;
        if v != 9 && v != 11 {
            ns.push(v);
        }
    }
    unsafe {
        for n in ns {
            seed_both(&c, &r, 0x31415926);
            (c.intput)(n);
            (r.intput)(n);
        }
    }
}

// --------------------------------------------------------------------------
// rows 43-45: end-to-end pipelines
// --------------------------------------------------------------------------

#[test]
fn cfg_43_pipeline_binary() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 43);
        let mut p = Pair::empty(&c, &r, 16, 8, 0, KeyKind::Inline, "row43");
        p.put_default();
        p.check("default slot");
        let mut keys = Vec::new();
        for i in 0..300u64 {
            let key = (rng.next_u64() | 1).to_le_bytes().to_vec();
            keys.push(key.clone());
            p.put_bin(&key, &i.to_le_bytes());
            p.check(&format!("pipeline put {i}"));
        }
        for k in keys.iter().take(100) {
            p.get_bin(k);
            p.check("pipeline get");
        }
        for k in keys.iter().take(100) {
            p.del_bin(k, 0);
            p.check("pipeline del");
        }
        p.free();
    }
}

#[test]
fn cfg_44_pipeline_strdup() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 44);
        let mut p = Pair::shmode(&c, &r, 16, 8, 1, 2, KeyKind::Pointer, "row44");
        let keys: Vec<CString> = (0..300).map(|_| rng.ascii_len(1, 25)).collect();
        for (i, k) in keys.iter().enumerate() {
            p.put_str(k, &(i as u64).to_le_bytes());
            p.check(&format!("strdup pipeline put {i}"));
        }
        for k in keys.iter().take(100) {
            p.get_str(k);
            p.check("strdup pipeline get");
        }
        for k in keys.iter().take(100) {
            p.del_str(k);
            p.check("strdup pipeline del");
        }
        p.free();
    }
}

#[test]
fn cfg_45_pipeline_arena() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 45);
        let mut p = Pair::shmode(&c, &r, 16, 8, 1, 3, KeyKind::Pointer, "row45");
        let keys: Vec<CString> = (0..300).map(|_| rng.ascii_len(1, 25)).collect();
        for (i, k) in keys.iter().enumerate() {
            p.put_str(k, &(i as u64).to_le_bytes());
            p.check(&format!("arena pipeline put {i}"));
        }
        for k in keys.iter().take(100) {
            p.get_str(k);
            p.check("arena pipeline get");
        }
        for k in keys.iter().take(100) {
            p.del_str(k);
            p.check("arena pipeline del");
        }
        p.free();
    }
}

// --------------------------------------------------------------------------
// row 46: hmput_default applied to a *length-0* array produced by arrgrowf
// (the `stbds_header(...)->length == 0` half of the `hmput_default` guard,
// which the NULL-argument half hides)
// --------------------------------------------------------------------------

#[test]
fn cfg_46_hmput_default_on_empty_array() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [8usize, 16, 40] {
            seed_both(&c, &r, 0x31415926);
            // a bare dynamic array: length == 0, capacity == 4, no hash table
            let ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            assert_eq!(header(ca).length, 0);
            assert_eq!(header(ra).length, 0);
            let ct = (c.hmput_default)((ca as *mut u8).add(elemsize) as *mut c_void, elemsize);
            let rt = (r.hmput_default)((ra as *mut u8).add(elemsize) as *mut c_void, elemsize);
            assert_same(
                &format!("hmput_default on empty array, elemsize={elemsize}"),
                &snapshot_map(ct, elemsize, 4, KeyKind::Inline),
                &snapshot_map(rt, elemsize, 4, KeyKind::Inline),
            );
            assert_eq!(map_header(ct, elemsize).length, 1);
            assert_eq!(map_header(rt, elemsize).length, 1);
            // and it must then work as a normal map
            let mut p = Pair {
                c: &c,
                r: &r,
                ct,
                rt,
                elemsize,
                keysize: 4,
                mode: 0,
                kind: KeyKind::Inline,
                label: format!("row46 elemsize={elemsize}"),
            };
            let mut rng = Rng::new(SEED ^ 46 ^ elemsize as u64);
            for i in 0..60u32 {
                let val = rng.bytes(elemsize - 4);
                p.put_bin(&k4(rng.next_u32()), &val);
                let _ = i;
            }
            p.check("puts after hmput_default on empty array");
            p.free();
        }
    }
}

// --------------------------------------------------------------------------
// row 47: stbds_temp_key (the scratch pointer the `shputs`/`hmputs` macros use)
// --------------------------------------------------------------------------

#[test]
fn cfg_47_temp_key_written_by_hmput_key() {
    let (c, r) = libs();
    unsafe {
        // 3 keys keeps used_count below the growth threshold (6 of 8 slots), so
        // `temp_key` is always a value both implementations actually wrote.
        for sh_mode in [1, 2, 3] {
            for mode in [1i32, 2] {
                seed_both(&c, &r, 0x31415926);
                let mut p = Pair::shmode(
                    &c,
                    &r,
                    16,
                    8,
                    mode,
                    sh_mode,
                    KeyKind::Pointer,
                    &format!("row47 sh={sh_mode} mode={mode}"),
                );
                let keys: Vec<CString> = ["alpha", "b", "gamma-delta-epsilon"]
                    .iter()
                    .map(|s| CString::new(*s).unwrap())
                    .collect();
                for (i, k) in keys.iter().enumerate() {
                    p.put_str(k, &(i as u64).to_le_bytes());
                    p.check_temp_key(k, &format!("fresh insert {i}"));
                    p.check(&format!("insert {i}"));
                }
                // re-putting an existing key must refresh temp_key too
                for (i, k) in keys.iter().enumerate().rev() {
                    p.put_str(k, &(100 + i as u64).to_le_bytes());
                    p.check_temp_key(k, &format!("re-put {i}"));
                    p.check(&format!("re-put {i}"));
                }
                p.free();
            }
        }
    }
}
