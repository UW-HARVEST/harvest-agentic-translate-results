//! Phase B — value API (CONFIGS rows 15-37).
#![allow(unused_unsafe, dead_code, unsafe_op_in_unsafe_fn)]
mod common;
use common::*;
use std::os::raw::{c_char, c_double, c_int, c_void};

// ---------------------------------------------------------------- builders

unsafe fn build(l: &Lib, v: &V) -> *mut JsonT {
    match v {
        V::Null => sym!(l, "json_null", () -> *mut JsonT)(),
        V::Bool(true) => sym!(l, "json_true", () -> *mut JsonT)(),
        V::Bool(false) => sym!(l, "json_false", () -> *mut JsonT)(),
        V::Int(i) => sym!(l, "json_integer", (i64) -> *mut JsonT)(*i),
        V::Real(f) => sym!(l, "json_real", (c_double) -> *mut JsonT)(*f),
        V::Str(s) => {
            let f = sym!(l, "json_stringn", (*const c_char, usize) -> *mut JsonT);
            f(s.as_ptr() as *const c_char, s.len())
        }
        V::Arr(a) => {
            let arr = sym!(l, "json_array", () -> *mut JsonT)();
            let app = sym!(l, "json_array_append_new", (*mut JsonT, *mut JsonT) -> c_int);
            for e in a {
                let child = build(l, e);
                assert_eq!(app(arr, child), 0);
            }
            arr
        }
        V::Obj(m) => {
            let obj = sym!(l, "json_object", () -> *mut JsonT)();
            let set = sym!(l, "json_object_setn_new", (*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int);
            for (k, e) in m {
                let child = build(l, e);
                assert_eq!(set(obj, k.as_ptr() as *const c_char, k.len(), child), 0);
            }
            obj
        }
    }
}

const ANY: usize = JSON_ENCODE_ANY;

unsafe fn snap(l: &Lib, v: *mut JsonT) -> Option<Vec<u8>> {
    dumps(l, v, ANY | JSON_SORT_KEYS)
}
unsafe fn snap_ordered(l: &Lib, v: *mut JsonT) -> Option<Vec<u8>> {
    dumps(l, v, ANY)
}

// ------------------------------------------------------------------ row 15

#[test]
fn row15_object_basic() {
    let (c, r) = both();
    let mut rng = Rng::new(15);
    for trial in 0..300 {
        let n = rng.below(60);
        let keys: Vec<String> = (0..n)
            .map(|i| {
                if rng.below(4) == 0 {
                    String::new()
                } else {
                    format!("k{}", rng.below(40))
                }
            })
            .collect();
        let vals: Vec<i64> = (0..n).map(|_| rng.i64()).collect();
        let logs: Vec<Vec<String>> = [c, r]
            .iter()
            .map(|l| unsafe {
                let obj = sym!(l, "json_object", () -> *mut JsonT)();
                let set = sym!(l, "json_object_set_new", (*mut JsonT, *const c_char, *mut JsonT) -> c_int);
                let get = sym!(l, "json_object_get", (*const JsonT, *const c_char) -> *mut JsonT);
                let size = sym!(l, "json_object_size", (*const JsonT) -> usize);
                let del = sym!(l, "json_object_del", (*mut JsonT, *const c_char) -> c_int);
                let clear = sym!(l, "json_object_clear", (*mut JsonT) -> c_int);
                let ival = sym!(l, "json_integer_value", (*const JsonT) -> i64);
                let integer = sym!(l, "json_integer", (i64) -> *mut JsonT);
                let mut log = Vec::new();
                for (k, v) in keys.iter().zip(&vals) {
                    let ck = cstr(k);
                    log.push(format!("set={}", set(obj, ck.as_ptr(), integer(*v))));
                    log.push(format!("size={}", size(obj)));
                }
                log.push(format!("dump={:?}", snap_ordered(l, obj)));
                for k in keys.iter() {
                    let ck = cstr(k);
                    let g = get(obj, ck.as_ptr());
                    log.push(if g.is_null() {
                        format!("get({})=NULL", k)
                    } else {
                        format!("get({})={}", k, ival(g))
                    });
                }
                // delete half
                for k in keys.iter().step_by(2) {
                    let ck = cstr(k);
                    log.push(format!("del={} size={}", del(obj, ck.as_ptr()), size(obj)));
                }
                log.push(format!("dump2={:?}", snap_ordered(l, obj)));
                log.push(format!("clear={} size={}", clear(obj), size(obj)));
                log.push(format!("dump3={:?}", snap_ordered(l, obj)));
                decref(l, obj);
                log
            })
            .collect();
        assert_eq!(logs[0], logs[1], "row15 trial {}", trial);
    }
}

// ------------------------------------------------------------------ row 16

#[test]
fn row16_object_setn_variants() {
    let (c, r) = both();
    let raw_keys: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"abc".to_vec(),
        vec![b'a', 0, b'b'],
        vec![0],
        "caf\u{e9}".as_bytes().to_vec(),
        "\u{4e2d}\u{6587}".as_bytes().to_vec(),
        vec![0x80],
        vec![0xC0, 0x80],
        vec![0xED, 0xA0, 0x80],
        vec![0xF4, 0x90, 0x80, 0x80],
        vec![0xE2, 0x82],
        b"longkeylongkeylongkeylongkey".to_vec(),
    ];
    for k in &raw_keys {
        for klen in 0..=k.len() {
            let logs: Vec<Vec<String>> = [c, r]
                .iter()
                .map(|l| unsafe {
                    let mut log = Vec::new();
                    for checked in [false, true] {
                        let obj = sym!(l, "json_object", () -> *mut JsonT)();
                        let integer = sym!(l, "json_integer", (i64) -> *mut JsonT);
                        let rc = if checked {
                            let f = sym!(l, "json_object_setn_new", (*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int);
                            f(obj, k.as_ptr() as *const c_char, klen, integer(7))
                        } else {
                            let f = sym!(l, "json_object_setn_new_nocheck", (*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int);
                            f(obj, k.as_ptr() as *const c_char, klen, integer(7))
                        };
                        let size = sym!(l, "json_object_size", (*const JsonT) -> usize);
                        let getn = sym!(l, "json_object_getn", (*const JsonT, *const c_char, usize) -> *mut JsonT);
                        log.push(format!(
                            "checked={} rc={} size={} getn={} dumpnc={:?}",
                            checked,
                            rc,
                            size(obj),
                            !getn(obj, k.as_ptr() as *const c_char, klen).is_null(),
                            dumps(l, obj, 0)
                        ));
                        // key_len must be preserved by the iterator
                        let iter = sym!(l, "json_object_iter", (*mut JsonT) -> *mut c_void);
                        let ikey = sym!(l, "json_object_iter_key", (*mut c_void) -> *const c_char);
                        let iklen = sym!(l, "json_object_iter_key_len", (*mut c_void) -> usize);
                        let it = iter(obj);
                        if !it.is_null() {
                            let n = iklen(it);
                            let kp = ikey(it) as *const u8;
                            log.push(format!(
                                "iterkey={:02x?} len={}",
                                std::slice::from_raw_parts(kp, n),
                                n
                            ));
                        } else {
                            log.push("iter=NULL".into());
                        }
                        decref(l, obj);
                    }
                    log
                })
                .collect();
            assert_eq!(logs[0], logs[1], "row16 key={:02x?} klen={}", k, klen);
        }
    }
}

// ------------------------------------------------------------------ row 17

#[test]
fn row17_object_getn_deln() {
    let (c, r) = both();
    let keys: Vec<&[u8]> = vec![
        b"a", b"ab", b"abc", b"abcd", b"b", b"", b"aa", b"aab",
    ];
    let logs: Vec<Vec<String>> = [c, r]
        .iter()
        .map(|l| unsafe {
            let obj = sym!(l, "json_object", () -> *mut JsonT)();
            let setn = sym!(l, "json_object_setn_new", (*mut JsonT, *const c_char, usize, *mut JsonT) -> c_int);
            let getn = sym!(l, "json_object_getn", (*const JsonT, *const c_char, usize) -> *mut JsonT);
            let deln = sym!(l, "json_object_deln", (*mut JsonT, *const c_char, usize) -> c_int);
            let integer = sym!(l, "json_integer", (i64) -> *mut JsonT);
            let ival = sym!(l, "json_integer_value", (*const JsonT) -> i64);
            let mut log = Vec::new();
            for (i, k) in keys.iter().enumerate() {
                setn(obj, k.as_ptr() as *const c_char, k.len(), integer(i as i64));
            }
            log.push(format!("dump={:?}", snap(l, obj)));
            for k in b"abcdz" {
                for klen in 0..5usize {
                    let buf = b"abcd";
                    let g = getn(obj, buf.as_ptr() as *const c_char, klen);
                    log.push(format!(
                        "getn(abcd,{})={}",
                        klen,
                        if g.is_null() {
                            "NULL".to_string()
                        } else {
                            ival(g).to_string()
                        }
                    ));
                    let _ = k;
                }
            }
            for klen in 0..5usize {
                let buf = b"abcd";
                log.push(format!(
                    "deln(abcd,{})={}",
                    klen,
                    deln(obj, buf.as_ptr() as *const c_char, klen)
                ));
                log.push(format!("dump={:?}", snap(l, obj)));
            }
            decref(l, obj);
            log
        })
        .collect();
    assert_eq!(logs[0], logs[1], "row17");
}

// ------------------------------------------------------------------ row 18

#[test]
fn row18_object_iteration() {
    let (c, r) = both();
    let mut rng = Rng::new(18);
    for trial in 0..200 {
        let n = rng.below(50);
        let keys: Vec<String> = (0..n).map(|i| format!("k{}", rng.below(30))).collect();
        let dels: Vec<usize> = (0..rng.below(10)).map(|_| rng.below(n.max(1))).collect();
        let logs: Vec<Vec<String>> = [c, r]
            .iter()
            .map(|l| unsafe {
                let obj = sym!(l, "json_object", () -> *mut JsonT)();
                let set = sym!(l, "json_object_set_new", (*mut JsonT, *const c_char, *mut JsonT) -> c_int);
                let del = sym!(l, "json_object_del", (*mut JsonT, *const c_char) -> c_int);
                let integer = sym!(l, "json_integer", (i64) -> *mut JsonT);
                let ival = sym!(l, "json_integer_value", (*const JsonT) -> i64);
                let iter = sym!(l, "json_object_iter", (*mut JsonT) -> *mut c_void);
                let iter_at = sym!(l, "json_object_iter_at", (*mut JsonT, *const c_char) -> *mut c_void);
                let iter_next = sym!(l, "json_object_iter_next", (*mut JsonT, *mut c_void) -> *mut c_void);
                let iter_key = sym!(l, "json_object_iter_key", (*mut c_void) -> *const c_char);
                let iter_klen = sym!(l, "json_object_iter_key_len", (*mut c_void) -> usize);
                let iter_val = sym!(l, "json_object_iter_value", (*mut c_void) -> *mut JsonT);
                let iter_set = sym!(l, "json_object_iter_set_new", (*mut JsonT, *mut c_void, *mut JsonT) -> c_int);
                let k2i = sym!(l, "json_object_key_to_iter", (*const c_char) -> *mut c_void);

                let mut log = Vec::new();
                for (i, k) in keys.iter().enumerate() {
                    let ck = cstr(k);
                    set(obj, ck.as_ptr(), integer(i as i64));
                }
                for d in &dels {
                    if *d < keys.len() {
                        let ck = cstr(&keys[*d]);
                        del(obj, ck.as_ptr());
                    }
                }
                // full walk
                let mut it = iter(obj);
                let mut idx = 0;
                while !it.is_null() {
                    let kp = iter_key(it);
                    let kl = iter_klen(it);
                    let kb = std::slice::from_raw_parts(kp as *const u8, kl).to_vec();
                    log.push(format!("[{}] {:?} = {}", idx, kb, ival(iter_val(it))));
                    // key_to_iter round trip
                    let it2 = k2i(kp);
                    log.push(format!("  k2i same = {}", it2 == it));
                    idx += 1;
                    it = iter_next(obj, it);
                }
                log.push(format!("count={}", idx));
                // iter_at + iter_set_new
                for k in keys.iter() {
                    let ck = cstr(k);
                    let it = iter_at(obj, ck.as_ptr());
                    if it.is_null() {
                        log.push(format!("iter_at({})=NULL", k));
                    } else {
                        log.push(format!(
                            "iter_at({}) rc={}",
                            k,
                            iter_set(obj, it, integer(-1))
                        ));
                    }
                }
                log.push(format!("dump={:?}", snap(l, obj)));
                decref(l, obj);
                log
            })
            .collect();
        assert_eq!(logs[0], logs[1], "row18 trial {}", trial);
    }
}

// -------------------------------------------------------------- rows 19-22

fn update_trials(name: &str, fname: &'static str) {
    let (c, r) = both();
    let mut rng = Rng::new(19);
    for trial in 0..300 {
        let a = rand_value(&mut rng, 3);
        let b = rand_value(&mut rng, 3);
        // ensure objects some of the time
        let (va, vb) = match (trial % 4, &a, &b) {
            (0, _, _) => (
                V::Obj(vec![
                    ("x".into(), V::Int(1)),
                    ("y".into(), V::Obj(vec![("z".into(), V::Int(2))])),
                ]),
                V::Obj(vec![
                    ("y".into(), V::Obj(vec![("w".into(), V::Int(3))])),
                    ("q".into(), V::Int(4)),
                ]),
            ),
            _ => (a.clone(), b.clone()),
        };
        let logs: Vec<Vec<String>> = [c, r]
            .iter()
            .map(|l| unsafe {
                let ja = build(l, &va);
                let jb = build(l, &vb);
                let f = match fname {
                    "json_object_update" => sym!(l, "json_object_update", (*mut JsonT, *mut JsonT) -> c_int),
                    "json_object_update_existing" => sym!(l, "json_object_update_existing", (*mut JsonT, *mut JsonT) -> c_int),
                    "json_object_update_missing" => sym!(l, "json_object_update_missing", (*mut JsonT, *mut JsonT) -> c_int),
                    _ => sym!(l, "json_object_update_recursive", (*mut JsonT, *mut JsonT) -> c_int),
                };
                let rc = f(ja, jb);
                let log = vec![
                    format!("rc={}", rc),
                    format!("a={:?}", snap(l, ja)),
                    format!("b={:?}", snap(l, jb)),
                ];
                decref(l, ja);
                decref(l, jb);
                log
            })
            .collect();
        assert_eq!(logs[0], logs[1], "{} trial {}", name, trial);
    }
}

#[test]
fn row19_object_update() {
    update_trials("row19", "json_object_update");
}
#[test]
fn row20_object_update_existing() {
    update_trials("row20", "json_object_update_existing");
}
#[test]
fn row21_object_update_missing() {
    update_trials("row21", "json_object_update_missing");
}
#[test]
fn row22_object_update_recursive() {
    update_trials("row22", "json_object_update_recursive");
    // also drive do_object_update_recursive directly with an explicit parents set
    let (c, r) = both();
    let mut rng = Rng::new(220);
    for trial in 0..200 {
        let va = rand_value(&mut rng, 3);
        let vb = rand_value(&mut rng, 3);
        let logs: Vec<Vec<String>> = [c, r]
            .iter()
            .map(|l| unsafe {
                let ja = build(l, &va);
                let jb = build(l, &vb);
                let mut ht = HashTable::zeroed();
                let init = sym!(l, "hashtable_init", (*mut HashTable) -> c_int);
                let close = sym!(l, "hashtable_close", (*mut HashTable));
                let f = sym!(l, "do_object_update_recursive", (*mut JsonT, *mut JsonT, *mut HashTable) -> c_int);
                assert_eq!(init(&mut ht), 0);
                let rc = f(ja, jb, &mut ht);
                close(&mut ht);
                let log = vec![format!("rc={}", rc), format!("a={:?}", snap(l, ja))];
                decref(l, ja);
                decref(l, jb);
                log
            })
            .collect();
        assert_eq!(logs[0], logs[1], "do_object_update_recursive trial {}", trial);
    }
}

// -------------------------------------------------------------- rows 23-26

#[test]
fn row23_row24_row25_row26_arrays() {
    let (c, r) = both();
    let mut rng = Rng::new(23);
    for trial in 0..400 {
        let n = rng.below(120);
        let vals: Vec<i64> = (0..n).map(|_| rng.i64()).collect();
        // random index programme
        let sets: Vec<(usize, i64)> = (0..rng.below(20))
            .map(|_| (rng.below(n + 3), rng.i64()))
            .collect();
        let inserts: Vec<(usize, i64)> = (0..rng.below(20))
            .map(|_| (rng.below(n + 3), rng.i64()))
            .collect();
        let removes: Vec<usize> = (0..rng.below(20)).map(|_| rng.below(n + 3)).collect();
        let other_n = rng.below(10);
        let logs: Vec<Vec<String>> = [c, r]
            .iter()
            .map(|l| unsafe {
                let arr = sym!(l, "json_array", () -> *mut JsonT)();
                let integer = sym!(l, "json_integer", (i64) -> *mut JsonT);
                let app = sym!(l, "json_array_append_new", (*mut JsonT, *mut JsonT) -> c_int);
                let get = sym!(l, "json_array_get", (*const JsonT, usize) -> *mut JsonT);
                let size = sym!(l, "json_array_size", (*const JsonT) -> usize);
                let setn = sym!(l, "json_array_set_new", (*mut JsonT, usize, *mut JsonT) -> c_int);
                let ins = sym!(l, "json_array_insert_new", (*mut JsonT, usize, *mut JsonT) -> c_int);
                let rem = sym!(l, "json_array_remove", (*mut JsonT, usize) -> c_int);
                let clear = sym!(l, "json_array_clear", (*mut JsonT) -> c_int);
                let extend = sym!(l, "json_array_extend", (*mut JsonT, *mut JsonT) -> c_int);
                let ival = sym!(l, "json_integer_value", (*const JsonT) -> i64);

                let mut log = Vec::new();
                for v in &vals {
                    log.push(format!("app={} size={}", app(arr, integer(*v)), size(arr)));
                }
                for i in 0..size(arr) + 3 {
                    let g = get(arr, i);
                    log.push(format!(
                        "get({})={}",
                        i,
                        if g.is_null() {
                            "NULL".into()
                        } else {
                            ival(g).to_string()
                        }
                    ));
                }
                log.push(format!("get(MAX)={}", get(arr, usize::MAX).is_null()));
                for (i, v) in &sets {
                    log.push(format!("set({})={}", i, setn(arr, *i, integer(*v))));
                }
                for (i, v) in &inserts {
                    log.push(format!(
                        "ins({})={} size={}",
                        i,
                        ins(arr, *i, integer(*v)),
                        size(arr)
                    ));
                }
                for i in &removes {
                    log.push(format!("rem({})={} size={}", i, rem(arr, *i), size(arr)));
                }
                log.push(format!("dump={:?}", snap_ordered(l, arr)));
                let other = sym!(l, "json_array", () -> *mut JsonT)();
                for j in 0..other_n {
                    app(other, integer(-(j as i64) - 1));
                }
                log.push(format!("extend={} dump={:?}", extend(arr, other), snap_ordered(l, arr)));
                log.push(format!("extend_self={}", extend(arr, arr)));
                log.push(format!("dump={:?}", snap_ordered(l, arr)));
                log.push(format!("clear={} size={}", clear(arr), size(arr)));
                log.push(format!("clear_again={}", clear(arr)));
                decref(l, other);
                decref(l, arr);
                log
            })
            .collect();
        assert_eq!(logs[0], logs[1], "arrays trial {}", trial);
    }
}

// ------------------------------------------------------------- rows 27-29

#[test]
fn row27_row28_row29_strings() {
    let (c, r) = both();
    let mut raws: Vec<Vec<u8>> = interesting_strings()
        .iter()
        .map(|s| s.as_bytes().to_vec())
        .collect();
    raws.extend([
        vec![],
        vec![0],
        vec![b'a', 0, b'b'],
        vec![0x80],
        vec![0xC0, 0x80],
        vec![0xED, 0xA0, 0x80],
        vec![0xF4, 0x90, 0x80, 0x80],
        vec![0xE2, 0x82],
        vec![0xFF, 0xFE],
    ]);
    let mut rng = Rng::new(27);
    for _ in 0..600 {
        let n = rng.below(20);
        raws.push((0..n).map(|_| rng.next_u32() as u8).collect());
    }

    for raw in &raws {
        for len in [raw.len(), raw.len().saturating_sub(1), 0] {
            let logs: Vec<Vec<String>> = [c, r]
                .iter()
                .map(|l| unsafe {
                    let mut log = Vec::new();
                    let sval = sym!(l, "json_string_value", (*const JsonT) -> *const c_char);
                    let slen = sym!(l, "json_string_length", (*const JsonT) -> usize);
                    let p = raw.as_ptr() as *const c_char;

                    // json_stringn / json_stringn_nocheck
                    for nocheck in [false, true] {
                        let s = if nocheck {
                            sym!(l, "json_stringn_nocheck", (*const c_char, usize) -> *mut JsonT)(p, len)
                        } else {
                            sym!(l, "json_stringn", (*const c_char, usize) -> *mut JsonT)(p, len)
                        };
                        if s.is_null() {
                            log.push(format!("stringn nocheck={} => NULL", nocheck));
                        } else {
                            let n = slen(s);
                            let body =
                                std::slice::from_raw_parts(sval(s) as *const u8, n).to_vec();
                            log.push(format!(
                                "stringn nocheck={} type={} len={} body={:02x?} dump={:?}",
                                nocheck,
                                (*s).typ,
                                n,
                                body,
                                dumps(l, s, ANY)
                            ));
                            // setters
                            for (i, other) in [b"".as_ref(), b"z".as_ref(), b"longer string".as_ref()]
                                .iter()
                                .enumerate()
                            {
                                let rc = sym!(l, "json_string_setn", (*mut JsonT, *const c_char, usize) -> c_int)(
                                    s, other.as_ptr() as *const c_char, other.len());
                                log.push(format!("setn{}={} dump={:?}", i, rc, dumps(l, s, ANY)));
                                let rc2 = sym!(l, "json_string_setn_nocheck", (*mut JsonT, *const c_char, usize) -> c_int)(
                                    s, raw.as_ptr() as *const c_char, len);
                                log.push(format!("setnnc{}={} len={}", i, rc2, slen(s)));
                            }
                            decref(l, s);
                        }
                    }

                    // NUL-terminated variants only when there is no interior NUL
                    if !raw.contains(&0) {
                        let cs = std::ffi::CString::new(raw.clone()).unwrap();
                        for nocheck in [false, true] {
                            let s = if nocheck {
                                sym!(l, "json_string_nocheck", (*const c_char) -> *mut JsonT)(cs.as_ptr())
                            } else {
                                sym!(l, "json_string", (*const c_char) -> *mut JsonT)(cs.as_ptr())
                            };
                            if s.is_null() {
                                log.push(format!("string nocheck={} => NULL", nocheck));
                            } else {
                                log.push(format!(
                                    "string nocheck={} len={} dump={:?}",
                                    nocheck,
                                    slen(s),
                                    dumps(l, s, ANY)
                                ));
                                let rc = sym!(l, "json_string_set", (*mut JsonT, *const c_char) -> c_int)(s, cs.as_ptr());
                                let rc2 = sym!(l, "json_string_set_nocheck", (*mut JsonT, *const c_char) -> c_int)(s, cs.as_ptr());
                                log.push(format!("set={} setnc={} len={}", rc, rc2, slen(s)));
                                decref(l, s);
                            }
                        }
                    }

                    // jsonp_stringn_nocheck_own takes ownership of a jsonp_malloc'd buffer
                    {
                        let malloc = sym!(l, "jsonp_malloc", (usize) -> *mut c_void);
                        let buf = malloc(len + 1) as *mut u8;
                        if !buf.is_null() {
                            std::ptr::copy_nonoverlapping(raw.as_ptr(), buf, len);
                            *buf.add(len) = 0;
                            let s = sym!(l, "jsonp_stringn_nocheck_own", (*const c_char, usize) -> *mut JsonT)(
                                buf as *const c_char, len);
                            if s.is_null() {
                                log.push("own => NULL".into());
                            } else {
                                log.push(format!("own len={} dump={:?}", slen(s), dumps(l, s, ANY)));
                                decref(l, s);
                            }
                        } else {
                            log.push("own: malloc(0) NULL".into());
                        }
                    }
                    log
                })
                .collect();
            assert_eq!(logs[0], logs[1], "strings raw={:02x?} len={}", raw, len);
        }
    }
}

// ------------------------------------------------------------- rows 30-32

#[test]
fn row30_integers() {
    let (c, r) = both();
    let mut vals: Vec<i64> = interesting_ints().to_vec();
    let mut rng = Rng::new(30);
    for _ in 0..4096 {
        vals.push(rng.i64());
    }
    for v in &vals {
        let logs: Vec<Vec<String>> = [c, r]
            .iter()
            .map(|l| unsafe {
                let integer = sym!(l, "json_integer", (i64) -> *mut JsonT);
                let ival = sym!(l, "json_integer_value", (*const JsonT) -> i64);
                let iset = sym!(l, "json_integer_set", (*mut JsonT, i64) -> c_int);
                let nval = sym!(l, "json_number_value", (*const JsonT) -> c_double);
                let j = integer(*v);
                let mut log = vec![format!(
                    "type={} val={} num={:?} dump={:?}",
                    (*j).typ,
                    ival(j),
                    nval(j).to_bits(),
                    dumps(l, j, ANY)
                )];
                log.push(format!("set={} val={}", iset(j, -*v), ival(j)));
                decref(l, j);
                log
            })
            .collect();
        assert_eq!(logs[0], logs[1], "integer {}", v);
    }
}

#[test]
fn row31_reals() {
    let (c, r) = both();
    let mut vals: Vec<f64> = interesting_reals().to_vec();
    vals.extend([f64::NAN, f64::INFINITY, f64::NEG_INFINITY]);
    let mut rng = Rng::new(31);
    for _ in 0..4096 {
        vals.push(rng.f64());
    }
    for _ in 0..512 {
        vals.push(f64::from_bits(rng.next_u64()));
    }
    for v in &vals {
        let logs: Vec<Vec<String>> = [c, r]
            .iter()
            .map(|l| unsafe {
                let real = sym!(l, "json_real", (c_double) -> *mut JsonT);
                let rval = sym!(l, "json_real_value", (*const JsonT) -> c_double);
                let rset = sym!(l, "json_real_set", (*mut JsonT, c_double) -> c_int);
                let nval = sym!(l, "json_number_value", (*const JsonT) -> c_double);
                let j = real(*v);
                if j.is_null() {
                    return vec!["NULL".to_string()];
                }
                let mut log = vec![format!(
                    "type={} val={} num={} dump={:?}",
                    (*j).typ,
                    rval(j).to_bits(),
                    nval(j).to_bits(),
                    dumps(l, j, ANY)
                )];
                log.push(format!("set_nan={}", rset(j, f64::NAN)));
                log.push(format!("set_inf={}", rset(j, f64::INFINITY)));
                log.push(format!("set_ok={} val={}", rset(j, 1.5), rval(j).to_bits()));
                decref(l, j);
                log
            })
            .collect();
        assert_eq!(logs[0], logs[1], "real {:e}", v);
    }
}

#[test]
fn row32_singletons() {
    let (c, r) = both();
    let logs: Vec<Vec<String>> = [c, r]
        .iter()
        .map(|l| unsafe {
            let t = sym!(l, "json_true", () -> *mut JsonT)();
            let f = sym!(l, "json_false", () -> *mut JsonT)();
            let n = sym!(l, "json_null", () -> *mut JsonT)();
            let del = sym!(l, "json_delete", (*mut JsonT));
            let t2 = sym!(l, "json_true", () -> *mut JsonT)();
            let mut log = vec![
                format!("true type={} rc={}", (*t).typ, (*t).refcount),
                format!("false type={} rc={}", (*f).typ, (*f).refcount),
                format!("null type={} rc={}", (*n).typ, (*n).refcount),
                format!("true singleton={}", t == t2),
                format!("dumps={:?} {:?} {:?}", dumps(l, t, ANY), dumps(l, f, ANY), dumps(l, n, ANY)),
            ];
            // json_delete on the singletons is the C `default:` no-op arm
            del(t);
            del(f);
            del(n);
            log.push(format!(
                "after delete: {} {} {}",
                (*t).typ,
                (*f).typ,
                (*n).typ
            ));
            log
        })
        .collect();
    assert_eq!(logs[0], logs[1], "row32");
}

// ------------------------------------------------------------------ row 33

#[test]
fn row33_equal() {
    let (c, r) = both();
    let samples: Vec<V> = vec![
        V::Null,
        V::Bool(true),
        V::Bool(false),
        V::Int(0),
        V::Int(1),
        V::Int(-1),
        V::Real(0.0),
        V::Real(1.0),
        V::Real(-0.0),
        V::Str("".into()),
        V::Str("a".into()),
        V::Str("b".into()),
        V::Arr(vec![]),
        V::Arr(vec![V::Int(1)]),
        V::Arr(vec![V::Int(1), V::Int(2)]),
        V::Arr(vec![V::Int(2), V::Int(1)]),
        V::Obj(vec![]),
        V::Obj(vec![("a".into(), V::Int(1))]),
        V::Obj(vec![("a".into(), V::Int(2))]),
        V::Obj(vec![("b".into(), V::Int(1))]),
        V::Obj(vec![("a".into(), V::Int(1)), ("b".into(), V::Int(2))]),
        V::Obj(vec![("b".into(), V::Int(2)), ("a".into(), V::Int(1))]),
        V::Arr(vec![V::Obj(vec![("a".into(), V::Arr(vec![V::Int(1)]))])]),
    ];
    let logs: Vec<Vec<String>> = [c, r]
        .iter()
        .map(|l| unsafe {
            let eq = sym!(l, "json_equal", (*const JsonT, *const JsonT) -> c_int);
            let mut log = Vec::new();
            let built: Vec<*mut JsonT> = samples.iter().map(|v| build(l, v)).collect();
            for (i, a) in built.iter().enumerate() {
                log.push(format!("null-lhs {}", eq(std::ptr::null(), *a)));
                log.push(format!("null-rhs {}", eq(*a, std::ptr::null())));
                for (j, b) in built.iter().enumerate() {
                    log.push(format!("eq({},{})={}", i, j, eq(*a, *b)));
                }
            }
            log.push(format!("null-null {}", eq(std::ptr::null(), std::ptr::null())));
            for b in built {
                decref(l, b);
            }
            log
        })
        .collect();
    assert_eq!(logs[0], logs[1], "row33");
}

// ------------------------------------------------------------- rows 34-35

#[test]
fn row34_row35_copy() {
    let (c, r) = both();
    let mut rng = Rng::new(34);
    let mut samples: Vec<V> = vec![
        V::Null,
        V::Bool(true),
        V::Bool(false),
        V::Int(42),
        V::Real(1.5),
        V::Str("s".into()),
        V::Arr(vec![]),
        V::Obj(vec![]),
    ];
    for _ in 0..500 {
        samples.push(rand_value(&mut rng, 5));
    }
    for (i, v) in samples.iter().enumerate() {
        let logs: Vec<Vec<String>> = [c, r]
            .iter()
            .map(|l| unsafe {
                let copy = sym!(l, "json_copy", (*mut JsonT) -> *mut JsonT);
                let deep = sym!(l, "json_deep_copy", (*const JsonT) -> *mut JsonT);
                let do_deep = sym!(l, "do_deep_copy", (*const JsonT, *mut HashTable) -> *mut JsonT);
                let eq = sym!(l, "json_equal", (*const JsonT, *const JsonT) -> c_int);
                let j = build(l, v);
                let sc = copy(j);
                let dc = deep(j);
                let mut ht = HashTable::zeroed();
                sym!(l, "hashtable_init", (*mut HashTable) -> c_int)(&mut ht);
                let dd = do_deep(j, &mut ht);
                sym!(l, "hashtable_close", (*mut HashTable))(&mut ht);
                let mut log = vec![
                    format!("orig={:?}", snap(l, j)),
                    format!("shallow={:?} eq={}", snap(l, sc), eq(j, sc)),
                    format!("deep={:?} eq={}", snap(l, dc), eq(j, dc)),
                    format!("do_deep={:?} eq={}", snap(l, dd), eq(j, dd)),
                ];
                // shallow copy shares children
                if let V::Arr(a) = v {
                    if !a.is_empty() {
                        let get = sym!(l, "json_array_get", (*const JsonT, usize) -> *mut JsonT);
                        log.push(format!("shares_child={}", get(j, 0) == get(sc, 0)));
                    }
                }
                log.push(format!("copy(NULL)={}", copy(std::ptr::null_mut()).is_null()));
                log.push(format!("deep(NULL)={}", deep(std::ptr::null()).is_null()));
                decref(l, sc);
                decref(l, dc);
                decref(l, dd);
                decref(l, j);
                log
            })
            .collect();
        assert_eq!(logs[0], logs[1], "copy sample {}", i);
    }
}

// ------------------------------------------------------------------ row 36

#[test]
fn row36_loop_check() {
    let (c, r) = both();
    let logs: Vec<Vec<String>> = [c, r]
        .iter()
        .map(|l| unsafe {
            let f = sym!(l, "jsonp_loop_check", (*mut HashTable, *const JsonT, *mut c_char, usize, *mut usize) -> c_int);
            let mut ht = HashTable::zeroed();
            sym!(l, "hashtable_init", (*mut HashTable) -> c_int)(&mut ht);
            let a = sym!(l, "json_array", () -> *mut JsonT)();
            let b = sym!(l, "json_array", () -> *mut JsonT)();
            let mut log = Vec::new();
            for (tag, p) in [("a", a), ("b", b), ("a", a), ("b", b)] {
                let mut key = [0u8; 19];
                let mut klen: usize = 0;
                let rc = f(&mut ht, p, key.as_mut_ptr() as *mut c_char, 19, &mut klen);
                let ks = std::ffi::CStr::from_ptr(key.as_ptr() as *const c_char)
                    .to_bytes()
                    .len();
                log.push(format!("{}: rc={} klen_matches={}", tag, rc, klen == ks));
            }
            // key_len_out == NULL
            let mut key = [0u8; 19];
            let rc = f(&mut ht, a, key.as_mut_ptr() as *mut c_char, 19, std::ptr::null_mut());
            log.push(format!("null-out rc={}", rc));
            sym!(l, "hashtable_close", (*mut HashTable))(&mut ht);
            decref(l, a);
            decref(l, b);
            log
        })
        .collect();
    assert_eq!(logs[0], logs[1], "row36");
}

// ------------------------------------------------------------------ row 37

#[test]
fn row37_sprintf() {
    let (c, r) = both();
    let cases: Vec<(&str, i64, &str)> = vec![
        ("plain", 0, ""),
        ("", 0, ""),
        ("%d", 42, ""),
        ("%d-%s", -7, "tail"),
        ("%s", 0, ""),
        ("%s", 0, "caf\u{e9}"),
        ("%s", 0, "\u{4e2d}\u{6587}"),
        ("num=%d str=%s", i64::MAX, "xyz"),
        ("%s", 0, "0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789"),
        ("%.3s", 0, "abcdef"),
        ("%%", 0, ""),
        ("%5d|", 3, ""),
        ("%s", 0, "with \"quote\" and \\slash"),
    ];
    for (fmt, i, s) in cases {
        let cf = cstr(fmt);
        let cs = cstr(s);
        let logs: Vec<Vec<String>> = [c, r]
            .iter()
            .map(|l| unsafe {
                let f: libloading::Symbol<
                    unsafe extern "C" fn(*const c_char, ...) -> *mut JsonT,
                > = l.lib.get(b"json_sprintf\0").unwrap();
                let j = if fmt.contains("%d") && fmt.contains("%s") {
                    f(cf.as_ptr(), i as c_int, cs.as_ptr())
                } else if fmt.contains("%d") {
                    f(cf.as_ptr(), i as c_int)
                } else if fmt.contains('s') && fmt.contains('%') {
                    f(cf.as_ptr(), cs.as_ptr())
                } else {
                    f(cf.as_ptr())
                };
                let out = if j.is_null() {
                    vec!["NULL".to_string()]
                } else {
                    let v = vec![
                        format!("type={}", (*j).typ),
                        format!("len={}", sym!(l, "json_string_length", (*const JsonT) -> usize)(j)),
                        format!("dump={:?}", dumps(l, j, ANY)),
                    ];
                    decref(l, j);
                    v
                };
                out
            })
            .collect();
        assert_eq!(logs[0], logs[1], "json_sprintf({:?})", fmt);
    }

    // invalid UTF-8 through %s must be rejected identically
    let bad = [b"\x80".as_ref(), b"\xC0\x80".as_ref(), b"\xED\xA0\x80".as_ref()];
    for b in bad {
        let cf = cstr("%s");
        let cs = std::ffi::CString::new(b.to_vec()).unwrap();
        let mut out = Vec::new();
        for l in [c, r] {
            unsafe {
                let f: libloading::Symbol<
                    unsafe extern "C" fn(*const c_char, ...) -> *mut JsonT,
                > = l.lib.get(b"json_sprintf\0").unwrap();
                let j = f(cf.as_ptr(), cs.as_ptr());
                out.push(j.is_null());
                if !j.is_null() {
                    decref(l, j);
                }
            }
        }
        assert_eq!(out[0], out[1], "json_sprintf invalid utf8 {:02x?}", b);
    }
}
