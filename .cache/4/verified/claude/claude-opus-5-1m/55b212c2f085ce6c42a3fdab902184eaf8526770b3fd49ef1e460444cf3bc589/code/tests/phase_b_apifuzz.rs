//! Phase B — end-to-end randomised API-sequence differential fuzz.
//!
//! Drives the library the way a real consumer does: a pool of live values that
//! gets mutated by a long random sequence of public calls, with the full state
//! re-dumped after every step.  This is what catches divergences in the
//! *composed* pipeline (hashtable rehash order, array growth, refcount
//! bookkeeping, shared sub-trees) that per-function tests cannot see.
//!
//! Invariant: a value may only be inserted into a container that was created
//! *later*, so the graph stays acyclic — `json_equal` has no cycle detection and
//! would recurse forever otherwise.

mod common;
use common::tree::*;
use common::*;
use std::os::raw::c_char;

const STEPS: usize = 400;
const ROUNDS: usize = 6;

#[test]
fn apifuzz_random_operation_sequences() {
    diff("api operation sequence fuzz", |api, rec| unsafe {
        for round in 0..ROUNDS {
            let mut rng = Rng::new(0xF0000 + round as u64);
            // pool[i] is owned by us; it may only reference pool[j] with j < i
            let mut pool: Vec<*mut Json> = Vec::new();

            for step in 0..STEPS {
                // always keep at least one container around
                if pool.is_empty() {
                    let s = rand_container(&mut rng, 2);
                    pool.push(build(api, &s));
                }
                let target = rng.below(pool.len());
                let t = pool[target];
                let older = if target == 0 { None } else { Some(rng.below(target)) };

                match rng.below(18) {
                    0 => {
                        let s = rand_container(&mut rng, 3);
                        let v = build(api, &s);
                        rec.json("new", v);
                        pool.push(v);
                    }
                    1 => {
                        let s = rand_spec(&mut rng, 2);
                        let v = build(api, &s);
                        rec.tag_i("append_new", (api.json_array_append_new)(t, v) as i64);
                    }
                    2 => {
                        let n = (api.json_array_size)(t);
                        let idx = if n == 0 { 0 } else { rng.below(n + 2) };
                        let s = rand_spec(&mut rng, 2);
                        let v = build(api, &s);
                        rec.tag_i(
                            "insert_new",
                            (api.json_array_insert_new)(t, idx, v) as i64,
                        );
                    }
                    3 => {
                        let n = (api.json_array_size)(t);
                        let idx = if n == 0 { 0 } else { rng.below(n + 2) };
                        let s = rand_spec(&mut rng, 2);
                        let v = build(api, &s);
                        rec.tag_i("set_new", (api.json_array_set_new)(t, idx, v) as i64);
                    }
                    4 => {
                        let n = (api.json_array_size)(t);
                        let idx = if n == 0 { 0 } else { rng.below(n + 2) };
                        rec.tag_i("remove", (api.json_array_remove)(t, idx) as i64);
                    }
                    5 => {
                        let key = format!("k{}", rng.below(24));
                        let k = cs(&key);
                        let s = rand_spec(&mut rng, 2);
                        let v = build(api, &s);
                        rec.tag_i(
                            "obj_set_new",
                            (api.json_object_set_new)(t, k.as_ptr(), v) as i64,
                        );
                    }
                    6 => {
                        let key = format!("k{}", rng.below(24));
                        let k = cs(&key);
                        rec.tag_i("obj_del", (api.json_object_del)(t, k.as_ptr()) as i64);
                    }
                    7 => {
                        // share an *older* value (keeps the graph acyclic)
                        if let Some(o) = older {
                            let v = incref(api, pool[o]);
                            let key = format!("s{}", rng.below(8));
                            let k = cs(&key);
                            let r = if rng.below(2) == 0 {
                                (api.json_array_append_new)(t, v)
                            } else {
                                (api.json_object_set_new)(t, k.as_ptr(), v)
                            };
                            rec.tag_i("share", r as i64);
                            if r != 0 {
                                // not adopted: the callee already decreffed it
                            }
                        }
                    }
                    8 => {
                        if let Some(o) = older {
                            let which = rng.below(4);
                            let r = match which {
                                0 => (api.json_object_update)(t, pool[o]),
                                1 => (api.json_object_update_existing)(t, pool[o]),
                                2 => (api.json_object_update_missing)(t, pool[o]),
                                _ => (api.json_object_update_recursive)(t, pool[o]),
                            };
                            rec.tag_i(&format!("update{which}"), r as i64);
                        }
                    }
                    9 => {
                        if let Some(o) = older {
                            rec.tag_i("extend", (api.json_array_extend)(t, pool[o]) as i64);
                        }
                    }
                    10 => {
                        let c = (api.json_copy)(t);
                        rec.json("copy", c);
                        if !c.is_null() {
                            pool.push(c);
                        }
                    }
                    11 => {
                        let c = (api.json_deep_copy)(t);
                        rec.json("deep_copy", c);
                        if !c.is_null() {
                            pool.push(c);
                        }
                    }
                    12 => {
                        rec.tag_i("clear_obj", (api.json_object_clear)(t) as i64);
                        rec.tag_i("clear_arr", (api.json_array_clear)(t) as i64);
                    }
                    13 => {
                        // full round trip through the text form
                        let flags = [
                            0usize,
                            JSON_COMPACT,
                            JSON_SORT_KEYS,
                            JSON_ENSURE_ASCII,
                            json_indent(2),
                            JSON_ESCAPE_SLASH | JSON_SORT_KEYS,
                        ][rng.below(6)];
                        match dumps(api, t, flags) {
                            None => rec.line("rt_dump=NULL"),
                            Some(d) => {
                                rec.tag_bytes("rt_dump", &d);
                                let z = cbuf(&d);
                                let mut e = JsonError::patterned();
                                let re = (api.json_loads)(
                                    z.as_ptr() as *const c_char,
                                    JSON_DECODE_ANY,
                                    &mut e,
                                );
                                rec.json("rt_parsed", re);
                                rec.error("rt_err", &e);
                                rec.tag_i("rt_equal", (api.json_equal)(t, re) as i64);
                                if !re.is_null() {
                                    pool.push(re);
                                }
                            }
                        }
                    }
                    14 => {
                        // iterate the object completely
                        let mut it = (api.json_object_iter)(t);
                        let mut n = 0;
                        while !it.is_null() {
                            let k = (api.json_object_iter_key)(it);
                            let kl = (api.json_object_iter_key_len)(it);
                            rec.tag_bytes(
                                "it_key",
                                std::slice::from_raw_parts(k as *const u8, kl),
                            );
                            rec.json("it_val", (api.json_object_iter_value)(it));
                            it = (api.json_object_iter_next)(t, it);
                            n += 1;
                        }
                        rec.tag_i("it_n", n);
                    }
                    15 => {
                        // pairwise equality across the pool
                        for (i, a) in pool.iter().enumerate().take(8) {
                            rec.tag_i(
                                &format!("eq{i}"),
                                (api.json_equal)(*a, t) as i64,
                            );
                        }
                    }
                    16 => {
                        if pool.len() > 1 {
                            let idx = pool.len() - 1;
                            let v = pool.remove(idx);
                            rec.json("drop", v);
                            decref(api, v);
                        }
                    }
                    _ => {
                        // json_dumpb size probing
                        let flags = [0usize, JSON_COMPACT, JSON_SORT_KEYS][rng.below(3)];
                        let need = (api.json_dumpb)(t, std::ptr::null_mut(), 0, flags);
                        rec.tag_u("need", need);
                        let mut buf = vec![0xA5u8; need + 4];
                        let got = (api.json_dumpb)(
                            t,
                            buf.as_mut_ptr() as *mut c_char,
                            need,
                            flags,
                        );
                        rec.tag_u("got", got);
                        rec.tag_bytes("buf", &buf);
                    }
                }

                // record the whole pool state every few steps
                if step % 17 == 0 {
                    for (i, v) in pool.iter().enumerate() {
                        rec.json(&format!("p{i}"), *v);
                        rec.tag_u(&format!("p{i}.osize"), (api.json_object_size)(*v));
                        rec.tag_u(&format!("p{i}.asize"), (api.json_array_size)(*v));
                        match dumps(api, *v, JSON_ENCODE_ANY | JSON_SORT_KEYS) {
                            None => rec.line(&format!("p{i}.dump=NULL")),
                            Some(d) => rec.tag_bytes(&format!("p{i}.dump"), &d),
                        }
                    }
                }
            }

            // final state, then release everything
            for (i, v) in pool.iter().enumerate() {
                rec.json(&format!("final{i}"), *v);
                match dumps(api, *v, JSON_ENCODE_ANY | JSON_SORT_KEYS | json_indent(1)) {
                    None => rec.line(&format!("final{i}.dump=NULL")),
                    Some(d) => rec.tag_bytes(&format!("final{i}.dump"), &d),
                }
            }
            for v in pool {
                decref(api, v);
            }
        }
    });
}
