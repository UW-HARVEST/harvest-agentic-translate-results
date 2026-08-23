//! Shared JSON tree builder used by several test files.
#![allow(dead_code)]

use super::*;

#[derive(Clone, Debug)]
pub enum Spec {
    Null,
    True,
    False,
    Int(i64),
    Real(f64),
    /// raw bytes, built with `json_stringn_nocheck` so invalid UTF-8 is possible
    StrRaw(Vec<u8>),
    /// valid UTF-8, built with `json_stringn`
    Str(String),
    Arr(Vec<Spec>),
    Obj(Vec<(Vec<u8>, Spec)>),
}

pub unsafe fn build(api: &Api, s: &Spec) -> *mut Json {
    match s {
        Spec::Null => (api.json_null)(),
        Spec::True => (api.json_true)(),
        Spec::False => (api.json_false)(),
        Spec::Int(v) => (api.json_integer)(*v),
        Spec::Real(v) => (api.json_real)(*v),
        Spec::StrRaw(b) => (api.json_stringn_nocheck)(b.as_ptr() as *const c_char, b.len()),
        Spec::Str(t) => (api.json_stringn)(t.as_ptr() as *const c_char, t.len()),
        Spec::Arr(items) => {
            let a = (api.json_array)();
            for it in items {
                let v = build(api, it);
                (api.json_array_append_new)(a, v);
            }
            a
        }
        Spec::Obj(pairs) => {
            let o = (api.json_object)();
            for (k, v) in pairs {
                let vv = build(api, v);
                (api.json_object_setn_new_nocheck)(o, k.as_ptr() as *const c_char, k.len(), vv);
            }
            o
        }
    }
}

/// Random spec generator.  `depth` bounds recursion.
pub fn rand_spec(rng: &mut Rng, depth: u32) -> Spec {
    let pick = rng.below(if depth == 0 { 7 } else { 9 });
    match pick {
        0 => Spec::Null,
        1 => Spec::True,
        2 => Spec::False,
        3 => {
            let v = match rng.below(4) {
                0 => rng.range_i64(-100, 100),
                1 => rng.range_i64(i32::MIN as i64, i32::MAX as i64),
                2 => rng.next_u64() as i64,
                _ => *[i64::MIN, i64::MAX, 0, -1, 1, 1 << 53, -(1 << 53)]
                    .get(rng.below(7))
                    .unwrap(),
            };
            Spec::Int(v)
        }
        4 => Spec::Real(rng.f64_interesting()),
        5 => {
            let n = rng.below(12);
            Spec::Str(rng.utf8(n))
        }
        6 => {
            let n = rng.below(8);
            Spec::Str(rng.utf8(n))
        }
        7 => {
            let n = rng.below(6);
            let mut v = Vec::new();
            for _ in 0..n {
                v.push(rand_spec(rng, depth - 1));
            }
            Spec::Arr(v)
        }
        _ => {
            let n = rng.below(6);
            let mut v = Vec::new();
            for i in 0..n {
                let key = match rng.below(3) {
                    0 => format!("k{i}"),
                    1 => { let n = 1 + rng.below(5); rng.utf8(n) }
                    _ => format!("dup{}", rng.below(2)),
                };
                v.push((key.into_bytes(), rand_spec(rng, depth - 1)));
            }
            Spec::Obj(v)
        }
    }
}

/// A container spec (so `json_dumps` works without `JSON_ENCODE_ANY`).
pub fn rand_container(rng: &mut Rng, depth: u32) -> Spec {
    if rng.below(2) == 0 {
        let n = rng.below(7);
        let mut v = Vec::new();
        for _ in 0..n {
            v.push(rand_spec(rng, depth));
        }
        Spec::Arr(v)
    } else {
        let n = rng.below(7);
        let mut v = Vec::new();
        for i in 0..n {
            let key = match rng.below(3) {
                0 => format!("key{i}"),
                1 => { let n = 1 + rng.below(6); rng.utf8(n) }
                _ => format!("s{}", rng.below(3)),
            };
            v.push((key.into_bytes(), rand_spec(rng, depth)));
        }
        Spec::Obj(v)
    }
}
